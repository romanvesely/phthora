//! Random-access frame reader for a clip. A background thread decodes the frames the
//! caller says it will need next, so playback, scrubbing, reverse and ping-pong never
//! block the caller while the frames are already cached.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::decoder::{DecodeOptions, Decoder, Frame};
use crate::probe::MediaInfo;

/// Keep decoding forward instead of seeking when the wanted frame is at most this far ahead.
const MAX_SKIP: u64 = 60;
/// Upper bound for cached frame data.
const CACHE_BYTES: usize = 512 << 20;

struct State {
    /// frame indices the caller needs, most urgent first
    wanted: Vec<u64>,
    cache: HashMap<u64, Arc<Frame>>,
    stop: bool,
    error: Option<String>,
}

struct Shared {
    state: Mutex<State>,
    cv: Condvar,
}

pub struct ClipReader {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
    info: MediaInfo,
    size: (u32, u32),
    fps: f64,
    frames: u64,
}

impl ClipReader {
    /// Decode `path` scaled to `w` x `h`. `info` must come from [`crate::probe`].
    pub fn open(path: &Path, info: MediaInfo, w: u32, h: u32) -> ClipReader {
        let fps = if info.fps > 0.0 { info.fps } else { 30.0 };
        let frames = ((info.duration * fps).round() as u64).max(1);
        let cap = (CACHE_BYTES / (w as usize * h as usize * 4).max(1)).clamp(8, 256);
        let shared = Arc::new(Shared {
            state: Mutex::new(State { wanted: vec![0], cache: HashMap::new(), stop: false, error: None }),
            cv: Condvar::new(),
        });
        let worker = Worker { path: path.to_path_buf(), info: info.clone(), size: (w, h), fps, cap, shared: shared.clone() };
        let thread = std::thread::Builder::new().name("phthora-clip".into()).spawn(move || worker.run()).ok();
        ClipReader { shared, thread, info, size: (w, h), fps, frames }
    }

    pub fn info(&self) -> &MediaInfo {
        &self.info
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Frame rate frames are indexed at (the source rate).
    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn frame_count(&self) -> u64 {
        self.frames
    }

    /// Tell the reader which frames come next (most urgent first). Never blocks.
    pub fn want(&self, indices: &[u64]) {
        let mut s = self.shared.state.lock().unwrap();
        let v: Vec<u64> = indices.iter().map(|i| (*i).min(self.frames - 1)).collect();
        if s.wanted != v {
            s.wanted = v;
            self.shared.cv.notify_all();
        }
    }

    /// The frame if it is decoded already.
    pub fn get(&self, idx: u64) -> Option<Arc<Frame>> {
        self.shared.state.lock().unwrap().cache.get(&idx.min(self.frames - 1)).cloned()
    }

    /// Wait for a frame (up to 30 s). `ahead` are the frames needed after it.
    pub fn get_blocking(&self, idx: u64, ahead: &[u64]) -> Option<Arc<Frame>> {
        let idx = idx.min(self.frames - 1);
        let mut want = vec![idx];
        want.extend(ahead.iter().map(|i| (*i).min(self.frames - 1)));
        self.want(&want);
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut s = self.shared.state.lock().unwrap();
        loop {
            if let Some(f) = s.cache.get(&idx) {
                return Some(f.clone());
            }
            if s.error.is_some() || s.stop {
                return None;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                s.error.get_or_insert_with(|| format!("timed out waiting for frame {idx}"));
                return None;
            }
            s = self.shared.cv.wait_timeout(s, left).unwrap().0;
        }
    }

    pub fn error(&self) -> Option<String> {
        self.shared.state.lock().unwrap().error.clone()
    }
}

impl Drop for ClipReader {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().stop = true;
        self.shared.cv.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct Worker {
    path: PathBuf,
    info: MediaInfo,
    size: (u32, u32),
    fps: f64,
    cap: usize,
    shared: Arc<Shared>,
}

/// An open decoder and the absolute index of the next frame it yields.
struct Open {
    dec: Decoder,
    next: u64,
    last: Option<Arc<Frame>>,
}

impl Worker {
    fn run(self) {
        let mut open: Option<Open> = None;
        loop {
            // next missing frame, or sleep until the wish list changes
            let (target, start) = {
                let mut s = self.shared.state.lock().unwrap();
                loop {
                    if s.stop {
                        return;
                    }
                    if let Some(m) = s.wanted.iter().copied().find(|i| !s.cache.contains_key(i)) {
                        // reverse playback wants lower frames next: decode them in the same pass
                        let lowest = s
                            .wanted
                            .iter()
                            .copied()
                            .filter(|i| *i < m && m - *i < MAX_SKIP && !s.cache.contains_key(i))
                            .min()
                            .unwrap_or(m);
                        break (m, lowest);
                    }
                    s = self.shared.cv.wait(s).unwrap();
                }
            };
            let reuse = open.as_ref().is_some_and(|o| o.next <= start && target - o.next <= MAX_SKIP);
            if !reuse {
                // stop the old ffmpeg before starting the next one
                drop(open.take());
                let opts = DecodeOptions { start: start as f64 / self.fps, fps: Some(self.fps), queue: 4, ..Default::default() };
                match Decoder::open(&self.path, &self.info, self.size.0, self.size.1, opts) {
                    Ok(dec) => open = Some(Open { dec, next: start, last: None }),
                    Err(e) => {
                        self.fail(e.to_string());
                        return;
                    }
                }
            }
            let o = open.as_mut().unwrap();
            // read up to the target; frames on the way are kept if they are wanted
            loop {
                match o.dec.next_frame() {
                    Some(f) => {
                        let idx = o.next;
                        o.next += 1;
                        let f = Arc::new(f);
                        o.last = Some(f.clone());
                        let done = idx >= target;
                        let mut s = self.shared.state.lock().unwrap();
                        if done || s.wanted.contains(&idx) {
                            s.cache.insert(idx, f);
                            self.evict(&mut s);
                            self.shared.cv.notify_all();
                        }
                        if done || s.stop || !s.wanted.contains(&target) {
                            break;
                        }
                    }
                    None => {
                        // the clip has fewer frames than its duration suggests:
                        // the missing tail shows the last real frame
                        let err = o.dec.error();
                        let (next, last) = (o.next, o.last.take());
                        open = None;
                        match last {
                            Some(f) if target >= next => {
                                let mut s = self.shared.state.lock().unwrap();
                                for i in next..=target {
                                    s.cache.insert(i, f.clone());
                                }
                                self.evict(&mut s);
                                self.shared.cv.notify_all();
                            }
                            _ => {
                                self.fail(err.unwrap_or_else(|| format!("no frame {target}")));
                                return;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    fn evict(&self, s: &mut State) {
        if s.cache.len() <= self.cap {
            return;
        }
        let keep: std::collections::HashSet<u64> = s.wanted.iter().copied().collect();
        let first = s.wanted.first().copied().unwrap_or(0);
        let mut extra: Vec<u64> = s.cache.keys().copied().filter(|k| !keep.contains(k)).collect();
        // drop the frames farthest from the current position first
        extra.sort_by_key(|k| std::cmp::Reverse(k.abs_diff(first)));
        let n = s.cache.len() - self.cap;
        for k in extra.into_iter().take(n) {
            s.cache.remove(&k);
        }
    }

    fn fail(&self, msg: String) {
        let mut s = self.shared.state.lock().unwrap();
        s.error.get_or_insert(msg);
        self.shared.cv.notify_all();
    }
}
