//! Overlay clips (media effect, film textures) sampled by time, backed by the clip reader.

use phthora_gpu::assets::{Assets, VideoSource};
use phthora_media::{ClipReader, Frame};
use std::path::Path;
use std::sync::Arc;

pub struct ClipSource {
    reader: ClipReader,
    /// preview: never wait for the decoder, show the newest frame available
    realtime: bool,
    cur: Option<(u64, Arc<Frame>)>,
}

impl ClipSource {
    pub fn open(path: &Path, max_dim: u32, realtime: bool) -> Option<Self> {
        let info = phthora_media::probe(path).ok()?;
        let (w, h) = phthora_media::fit_size(info.width, info.height, max_dim);
        Some(Self { reader: ClipReader::open(path, info, w, h), realtime, cur: None })
    }

    fn index_for(&self, t: f64) -> u64 {
        let n = self.reader.frame_count();
        let d = n as f64 / self.reader.fps();
        ((t.rem_euclid(d) * self.reader.fps()).floor() as u64).min(n - 1)
    }
}

impl VideoSource for ClipSource {
    fn size(&self) -> (u32, u32) {
        self.reader.size()
    }
    fn duration(&self) -> f64 {
        self.reader.frame_count() as f64 / self.reader.fps()
    }
    fn frame_at(&mut self, t: f64) -> Option<(u64, &[u8])> {
        let idx = self.index_for(t);
        let n = self.reader.frame_count();
        if self.cur.as_ref().map(|(i, _)| *i) != Some(idx) {
            let ahead: Vec<u64> = (1..8).map(|k| (idx + k) % n).collect();
            // the first frame is worth waiting for, later ones are not (in the preview)
            let f = if self.realtime && self.cur.is_some() {
                let mut want = vec![idx];
                want.extend(&ahead);
                self.reader.want(&want);
                self.reader.get(idx)
            } else {
                self.reader.get_blocking(idx, &ahead)
            };
            if let Some(f) = f {
                self.cur = Some((idx, f));
            }
        }
        self.cur.as_ref().map(|(i, f)| (*i, f.data.as_slice()))
    }
}

/// Let the engine decode overlay clips and user images through ffmpeg.
pub fn install(assets: &mut Assets, realtime: bool) {
    if !phthora_media::ffmpeg_available() {
        return;
    }
    assets.video_opener = Some(Box::new(move |p: &Path| ClipSource::open(p, 1280, realtime).map(|c| Box::new(c) as Box<dyn VideoSource>)));
    assets.image_loader = Some(Box::new(|p: &Path| {
        phthora_media::load_image(p, 4096).ok().map(|i| (i.width, i.height, i.data))
    }));
}
