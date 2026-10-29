//! Person segmentation on the CPU: a small ONNX network that is downloaded on first use,
//! run through tract, plus a worker thread for live previews.

mod download;
mod model;
mod segmenter;
mod worker;

pub use download::{download, verify_file};
pub use model::{is_installed, model_path, models_dir, set_models_dir, ModelSpec, MODEL};
pub use segmenter::Segmenter;
pub use worker::Worker;

use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("download failed: {0}")]
    Download(String),
    #[error("download cancelled")]
    Cancelled,
    #[error("checksum mismatch: expected {expected}, got {actual}")]
    Checksum { expected: String, actual: String },
    #[error("model: {0}")]
    Model(String),
}

/// Person probability per pixel, 0..255, rows top-down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mask {
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
}

impl Mask {
    /// Bilinear resample (e.g. to the frame size for CPU use; the GPU samples the mask directly).
    pub fn resize(&self, w: u32, h: u32) -> Mask {
        let (sw, sh) = (self.w as usize, self.h as usize);
        let mut data = Vec::with_capacity((w * h) as usize);
        for y in 0..h as usize {
            let fy = ((y as f32 + 0.5) * sh as f32 / h as f32 - 0.5).clamp(0.0, sh as f32 - 1.0);
            let y0 = fy as usize;
            let y1 = (y0 + 1).min(sh - 1);
            let ty = fy - y0 as f32;
            for x in 0..w as usize {
                let fx = ((x as f32 + 0.5) * sw as f32 / w as f32 - 0.5).clamp(0.0, sw as f32 - 1.0);
                let x0 = fx as usize;
                let x1 = (x0 + 1).min(sw - 1);
                let tx = fx - x0 as f32;
                let g = |x: usize, y: usize| self.data[y * sw + x] as f32;
                let top = g(x0, y0) * (1.0 - tx) + g(x1, y0) * tx;
                let bottom = g(x0, y1) * (1.0 - tx) + g(x1, y1) * tx;
                data.push((top * (1.0 - ty) + bottom * ty + 0.5) as u8);
            }
        }
        Mask { w, h, data }
    }
}

/// What the model is doing right now, for a hint or download dialog in the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// not on disk: offer the download
    Missing,
    /// `download()` is running (bytes done, bytes total)
    Downloading { done: u64, total: u64 },
    /// on disk, not loaded yet (loads when an effect needs it)
    Available,
    Loading,
    Ready,
    /// loading failed (corrupt file, checksum): offer to download again
    Failed(String),
}

#[derive(Clone, Debug)]
enum Load {
    Idle,
    Loading,
    Ready,
    Failed(String),
}

struct State {
    download: Option<(u64, u64)>,
    load: Load,
}

static STATE: Mutex<State> = Mutex::new(State { download: None, load: Load::Idle });

fn set_load(l: Load) {
    STATE.lock().unwrap().load = l;
}

fn set_download(d: Option<(u64, u64)>) {
    STATE.lock().unwrap().download = d;
}

/// Current state of the model, cheap enough to call every frame.
pub fn status() -> Status {
    let s = STATE.lock().unwrap();
    if let Some((done, total)) = s.download {
        return Status::Downloading { done, total };
    }
    match &s.load {
        Load::Ready => Status::Ready,
        Load::Loading => Status::Loading,
        Load::Failed(e) => Status::Failed(e.clone()),
        Load::Idle => {
            drop(s);
            if is_installed() { Status::Available } else { Status::Missing }
        }
    }
}
