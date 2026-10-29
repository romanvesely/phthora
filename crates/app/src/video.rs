//! Video clips sampled by time, backed by the ffmpeg frame decoder.

use phthora_gpu::assets::{Assets, VideoSource};
use phthora_media::{DecodeOptions, Decoder, Frame, MediaInfo};
use std::path::{Path, PathBuf};

/// Streams a clip and returns the frame for any time. Sequential access (playback,
/// export) reads forward; jumps restart the decoder at the new position.
pub struct ClipSource {
    path: PathBuf,
    info: MediaInfo,
    size: (u32, u32),
    fps: f64,
    frames: u64,
    dec: Option<Decoder>,
    /// absolute frame index of the decoder's first frame
    base: u64,
    cur: Option<(u64, Frame)>,
}

impl ClipSource {
    pub fn open(path: &Path, max_dim: u32) -> Option<Self> {
        let info = phthora_media::probe(path).ok()?;
        let (w, h) = phthora_media::fit_size(info.width, info.height, max_dim);
        Self::with_size(path, info, w, h)
    }

    pub fn with_size(path: &Path, info: MediaInfo, w: u32, h: u32) -> Option<Self> {
        let fps = if info.fps > 0.0 { info.fps } else { 30.0 };
        let frames = ((info.duration * fps).round() as u64).max(1);
        Some(Self { path: path.to_path_buf(), info, size: (w, h), fps, frames, dec: None, base: 0, cur: None })
    }

    pub fn info(&self) -> &MediaInfo {
        &self.info
    }

    fn restart(&mut self, idx: u64) {
        self.dec = None;
        let opts = DecodeOptions { start: idx as f64 / self.fps, fps: Some(self.fps), looped: false, queue: 4, ..Default::default() };
        self.dec = Decoder::open(&self.path, &self.info, self.size.0, self.size.1, opts).ok();
        self.base = idx;
    }

    fn index_for(&self, t: f64) -> u64 {
        let d = self.frames as f64 / self.fps;
        ((t.rem_euclid(d) * self.fps).floor() as u64).min(self.frames - 1)
    }

    /// Frame for time t (seconds, wraps at the clip length).
    pub fn frame(&mut self, t: f64) -> Option<&Frame> {
        let idx = self.index_for(t);
        let have = self.cur.as_ref().map(|(i, _)| *i);
        if have != Some(idx) {
            let need_restart = match have {
                None => true,
                Some(h) => idx < h || idx > h + 90 || self.dec.is_none(),
            };
            if need_restart {
                self.restart(idx);
            }
            loop {
                let Some(dec) = self.dec.as_mut() else { break };
                match dec.next_frame() {
                    Some(f) => {
                        let abs = self.base + f.index;
                        if abs >= idx {
                            self.cur = Some((abs, f));
                            break;
                        }
                    }
                    None => {
                        // ran past the end: keep the last frame we have
                        self.dec = None;
                        break;
                    }
                }
            }
        }
        self.cur.as_ref().map(|(_, f)| f)
    }
}

impl VideoSource for ClipSource {
    fn size(&self) -> (u32, u32) {
        self.size
    }
    fn duration(&self) -> f64 {
        self.frames as f64 / self.fps
    }
    fn frame_at(&mut self, t: f64) -> Option<(u64, &[u8])> {
        self.frame(t)?;
        self.cur.as_ref().map(|(i, f)| (*i, f.data.as_slice()))
    }
}

/// Let the engine decode overlay clips and user images through ffmpeg.
pub fn install(assets: &mut Assets) {
    if !phthora_media::ffmpeg_available() {
        return;
    }
    assets.video_opener = Some(Box::new(|p: &Path| ClipSource::open(p, 1280).map(|c| Box::new(c) as Box<dyn VideoSource>)));
    assets.image_loader = Some(Box::new(|p: &Path| {
        phthora_media::load_image(p, 4096).ok().map(|i| (i.width, i.height, i.data))
    }));
}
