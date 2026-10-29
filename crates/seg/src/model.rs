//! Description of the person segmentation model and where it lives on disk.

use std::path::PathBuf;
use std::sync::Mutex;

/// A downloadable ONNX model.
#[derive(Debug)]
pub struct ModelSpec {
    pub name: &'static str,
    /// file name inside the models folder
    pub file: &'static str,
    /// pinned download location (https)
    pub url: &'static str,
    /// lowercase hex SHA-256 of the file
    pub sha256: &'static str,
    pub size: u64,
    /// network input size in pixels (the frame is squashed to it)
    pub input_w: usize,
    pub input_h: usize,
    /// input = (rgb / 255 - mean) / std
    pub mean: [f32; 3],
    pub std: [f32; 3],
}

/// MediaPipe Selfie Segmentation, landscape variant (Google, Apache-2.0), as ONNX.
/// One output channel: person probability 0..1 at the input size.
pub const MODEL: ModelSpec = ModelSpec {
    name: "Selfie Segmentation (landscape)",
    file: "selfie-segmentation-landscape.onnx",
    url: "https://huggingface.co/onnx-community/mediapipe_selfie_segmentation_landscape/resolve/2497d5bec26c626c7b3c4edc6e1fefc21b64f6c3/onnx/model.onnx",
    sha256: "7a0adcfdb1715d3b0ff0f61486d8a181c33f4a208343a5abfccbc6540e872d24",
    size: 462_338,
    input_w: 256,
    input_h: 144,
    mean: [0.0; 3],
    std: [1.0; 3],
};

static DIR_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Use another models folder (tests, custom locations); `None` restores the default.
pub fn set_models_dir(dir: Option<PathBuf>) {
    *DIR_OVERRIDE.lock().unwrap() = dir;
}

/// `$XDG_DATA_HOME/phthora/models` on Linux (same data folder the app uses for presets).
pub fn models_dir() -> PathBuf {
    if let Some(d) = DIR_OVERRIDE.lock().unwrap().clone() {
        return d;
    }
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("phthora").join("models")
}

/// Where the model is (or will be) stored.
pub fn model_path() -> PathBuf {
    models_dir().join(MODEL.file)
}

/// Whether a file of the right size is in place (cheap; the checksum is verified on load).
pub fn is_installed() -> bool {
    std::fs::metadata(model_path()).map(|m| m.len() == MODEL.size).unwrap_or(false)
}
