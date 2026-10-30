//! Streaming video decoder: ffmpeg child process -> raw RGBA frames over a
//! bounded channel (backpressure).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::error::{MediaError, Result};
use crate::image::decode_still;
use crate::probe::{MediaInfo, MediaKind};
use crate::util::{StderrTail, fps_arg, ffmpeg_cmd, passthrough_args, require_ffmpeg, secs_arg};

#[derive(Debug, Clone)]
pub struct DecodeOptions {
    /// Start offset in seconds.
    pub start: f64,
    /// End offset in seconds (exclusive); `None` = end of the file.
    pub end: Option<f64>,
    /// Resample to this constant rate. With a rate set, every pass yields
    /// exactly `round((end - start) * fps)` frames (last frame repeated if
    /// the source runs short, extra frames cut). Without it, frames come at
    /// the source rate (pts is only approximate for VFR files).
    pub fps: Option<f64>,
    /// Restart from `start` at the end of each pass.
    pub looped: bool,
    /// Capacity of the frame queue (decoder runs at most this far ahead).
    pub queue: usize,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: None,
            fps: None,
            looped: false,
            queue: 8,
        }
    }
}

/// One decoded frame, top-down RGBA8, straight alpha.
#[derive(Debug, Clone)]
pub struct Frame {
    /// Index within the current pass (0 at `start`).
    pub index: u64,
    /// Loop pass, starting at 0.
    pub pass: u32,
    /// Source time in seconds (`start + index / fps`).
    pub pts: f64,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

type ChildSlot = Arc<Mutex<Option<Child>>>;

struct Job {
    path: PathBuf,
    /// Forced libvpx decoder for vp8/vp9 (keeps alpha).
    vp_decoder: Option<&'static str>,
    opts: DecodeOptions,
    src_size: (u32, u32),
    out_size: (u32, u32),
    /// Frames per pass if known up front.
    planned: Option<u64>,
    pts_fps: f64,
    still: Option<Vec<u8>>,
}

pub struct Decoder {
    rx: Option<Receiver<Frame>>,
    stop: Arc<AtomicBool>,
    slot: ChildSlot,
    error: Arc<Mutex<Option<String>>>,
    thread: Option<JoinHandle<()>>,
    ended: bool,
    out_size: (u32, u32),
}

impl Decoder {
    /// Start decoding `path` scaled to `out_w` x `out_h`. `info` must come
    /// from [`crate::probe`] for the same file.
    pub fn open(
        path: &Path,
        info: &MediaInfo,
        out_w: u32,
        out_h: u32,
        opts: DecodeOptions,
    ) -> Result<Decoder> {
        require_ffmpeg()?;
        if out_w == 0 || out_h == 0 {
            return Err(MediaError::InvalidInput("output size must be non-zero".into()));
        }
        if let Some(f) = opts.fps
            && !(f > 0.0 && f.is_finite()) {
                return Err(MediaError::InvalidInput("fps must be positive".into()));
            }
        let is_image = info.kind == MediaKind::Image;
        let end = opts.end.unwrap_or(info.duration);
        let span = (end - opts.start).max(0.0);
        let planned = match opts.fps {
            Some(f) if is_image => Some(if opts.end.is_some() { (span * f).round().max(1.0) as u64 } else { 1 }),
            Some(f) => Some((span * f).round().max(1.0) as u64),
            None if is_image => Some(1),
            None => None,
        };
        let still = if is_image {
            let img = decode_still(path, info.width, info.height, out_w, out_h)?;
            Some(img.data)
        } else {
            None
        };
        let job = Job {
            path: path.to_path_buf(),
            vp_decoder: match info.codec.as_str() {
                "vp8" => Some("libvpx"),
                "vp9" => Some("libvpx-vp9"),
                _ => None,
            },
            src_size: (info.width, info.height),
            out_size: (out_w, out_h),
            planned,
            pts_fps: opts.fps.unwrap_or(info.fps),
            still,
            opts: opts.clone(),
        };

        let (tx, rx) = sync_channel(opts.queue.max(1));
        let stop = Arc::new(AtomicBool::new(false));
        let slot: ChildSlot = Arc::new(Mutex::new(None));
        let error = Arc::new(Mutex::new(None));
        let thread = {
            let (stop, slot, error) = (stop.clone(), slot.clone(), error.clone());
            std::thread::Builder::new()
                .name("phthora-decoder".into())
                .spawn(move || worker(job, tx, stop, slot, error))?
        };
        Ok(Decoder {
            rx: Some(rx),
            stop,
            slot,
            error,
            thread: Some(thread),
            ended: false,
            out_size: (out_w, out_h),
        })
    }

    /// Output frame size.
    pub fn size(&self) -> (u32, u32) {
        self.out_size
    }

    /// Next frame, blocking. `None` when the stream is over (never for a
    /// looped decoder unless decoding failed; check [`Decoder::error`]).
    pub fn next_frame(&mut self) -> Option<Frame> {
        match self.rx.as_ref()?.recv() {
            Ok(f) => Some(f),
            Err(_) => {
                self.ended = true;
                None
            }
        }
    }

    /// Next frame if one is ready; never blocks. Use [`Decoder::is_finished`]
    /// to tell "not yet" from "no more".
    pub fn try_next_frame(&mut self) -> Option<Frame> {
        match self.rx.as_ref()?.try_recv() {
            Ok(f) => Some(f),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.ended = true;
                None
            }
        }
    }

    /// True once the stream ended and all frames were taken.
    pub fn is_finished(&self) -> bool {
        self.ended
    }

    /// Error text (ffmpeg stderr tail) if decoding failed.
    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.rx = None;
        if let Some(c) = self.slot.lock().unwrap().as_mut() {
            let _ = c.kill();
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

enum Pass {
    Done(u64),
    Stopped,
    Failed(String),
}

fn worker(
    job: Job,
    tx: SyncSender<Frame>,
    stop: Arc<AtomicBool>,
    slot: ChildSlot,
    error: Arc<Mutex<Option<String>>>,
) {
    let mut pass = 0u32;
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let r = match &job.still {
            Some(buf) => emit_still(&job, buf, pass, &tx),
            None => run_pass(&job, pass, &tx, &stop, &slot, &error),
        };
        match r {
            Pass::Done(0) => {
                let mut e = error.lock().unwrap();
                if e.is_none() {
                    *e = Some("decoder produced no frames".into());
                }
                break;
            }
            Pass::Done(_) => {}
            Pass::Stopped => break,
            Pass::Failed(m) => {
                *error.lock().unwrap() = Some(m);
                break;
            }
        }
        if !job.opts.looped {
            break;
        }
        pass += 1;
    }
}

fn make_frame(job: &Job, index: u64, pass: u32, data: Vec<u8>) -> Frame {
    let pts = if job.pts_fps > 0.0 {
        job.opts.start + index as f64 / job.pts_fps
    } else {
        job.opts.start
    };
    Frame {
        index,
        pass,
        pts,
        width: job.out_size.0,
        height: job.out_size.1,
        data,
    }
}

fn emit_still(job: &Job, buf: &[u8], pass: u32, tx: &SyncSender<Frame>) -> Pass {
    let n = job.planned.unwrap_or(1);
    for i in 0..n {
        if tx.send(make_frame(job, i, pass, buf.to_vec())).is_err() {
            return Pass::Stopped;
        }
    }
    Pass::Done(n)
}

fn build_args(job: &Job) -> Vec<String> {
    let mut a: Vec<String> = vec!["-nostdin".into()];
    if let Some(dec) = job.vp_decoder {
        // The native vp8/vp9 decoders drop alpha; libvpx keeps it.
        a.extend(["-c:v".into(), dec.into()]);
    }
    if job.opts.start > 0.0 {
        a.extend(["-ss".into(), secs_arg(job.opts.start)]);
    }
    a.extend(["-i".into(), job.path.to_string_lossy().into_owned()]);
    a.extend(["-map", "0:v:0", "-an", "-sn"].map(String::from));
    let mut vf: Vec<String> = Vec::new();
    if let Some(f) = job.opts.fps {
        vf.push(format!("fps={}", fps_arg(f)));
    }
    if job.out_size != job.src_size {
        vf.push(format!("scale={}:{}", job.out_size.0, job.out_size.1));
    }
    if !vf.is_empty() {
        a.extend(["-vf".into(), vf.join(",")]);
    }
    a.extend(["-pix_fmt", "rgba"].map(String::from));
    a.extend(passthrough_args().map(String::from));
    if let Some(n) = job.planned {
        a.extend(["-frames:v".into(), n.to_string()]);
    } else if let Some(end) = job.opts.end {
        a.extend(["-t".into(), secs_arg(end - job.opts.start)]);
    }
    a.extend(["-f", "rawvideo", "-"].map(String::from));
    a
}

/// Read exactly `buf.len()` bytes; `false` on EOF (also mid-frame).
fn read_frame(r: &mut ChildStdout, buf: &mut [u8]) -> bool {
    let mut got = 0;
    while got < buf.len() {
        match r.read(&mut buf[got..]) {
            Ok(0) => return false,
            Ok(n) => got += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return false,
        }
    }
    true
}

fn run_pass(
    job: &Job,
    pass: u32,
    tx: &SyncSender<Frame>,
    stop: &AtomicBool,
    slot: &ChildSlot,
    error: &Mutex<Option<String>>,
) -> Pass {
    let mut cmd = ffmpeg_cmd();
    cmd.args(build_args(job))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Pass::Failed(format!("cannot start ffmpeg: {e}")),
    };
    let mut stdout = child.stdout.take().unwrap();
    let tail = StderrTail::spawn(child.stderr.take().unwrap());
    {
        let mut g = slot.lock().unwrap();
        if stop.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            return Pass::Stopped;
        }
        *g = Some(child);
    }

    let frame_len = job.out_size.0 as usize * job.out_size.1 as usize * 4;
    // With a planned count the newest frame is held back so it can be
    // repeated if the source runs short.
    let hold = job.planned.is_some();
    let mut held: Option<Vec<u8>> = None;
    let mut sent = 0u64;
    let mut stopped = false;
    let mut reached_eof = false;

    let send = |data: Vec<u8>, sent: &mut u64| -> bool {
        let ok = tx.send(make_frame(job, *sent, pass, data)).is_ok();
        if ok {
            *sent += 1;
        }
        ok
    };

    loop {
        if stop.load(Ordering::SeqCst) {
            stopped = true;
            break;
        }
        let have = sent + held.is_some() as u64;
        if job.planned.is_some_and(|n| have >= n) {
            break;
        }
        let mut buf = vec![0u8; frame_len];
        if !read_frame(&mut stdout, &mut buf) {
            reached_eof = true;
            break;
        }
        if hold {
            if let Some(prev) = held.replace(buf)
                && !send(prev, &mut sent) {
                    stopped = true;
                    break;
                }
        } else if !send(buf, &mut sent) {
            stopped = true;
            break;
        }
    }

    if !stopped && hold {
        // Flush the held frame, then pad up to the planned count.
        let target = job.planned.unwrap_or(0);
        if let Some(last) = held.take() {
            while sent + 1 < target {
                if !send(last.clone(), &mut sent) {
                    stopped = true;
                    break;
                }
            }
            if !stopped && sent < target.max(1) && !send(last, &mut sent) {
                stopped = true;
            }
        }
    }

    // Reap the process: kill unless it ended by itself.
    let child = slot.lock().unwrap().take();
    let mut status_msg = None;
    if let Some(mut c) = child {
        if reached_eof {
            if let Ok(st) = c.wait()
                && !st.success() {
                    status_msg = Some(st.to_string());
                }
        } else {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
    let stderr = tail.finish();
    if stopped || stop.load(Ordering::SeqCst) {
        return Pass::Stopped;
    }
    if let Some(st) = status_msg {
        let msg = format!("ffmpeg exited with {st}: {stderr}");
        if sent == 0 {
            return Pass::Failed(msg);
        }
        // Frames already delivered; keep the details for the caller.
        *error.lock().unwrap() = Some(msg);
    } else if sent == 0 && !stderr.is_empty() {
        return Pass::Failed(stderr);
    }
    Pass::Done(sent)
}
