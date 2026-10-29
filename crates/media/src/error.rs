use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("ffmpeg/ffprobe not found in PATH")]
    FfmpegMissing,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not probe {path}: {message}")]
    Probe { path: PathBuf, message: String },
    #[error("{0}")]
    InvalidInput(String),
    /// ffmpeg failed; `stderr` holds the last lines of its error output.
    #[error("ffmpeg failed ({context}): {stderr}")]
    Ffmpeg { context: String, stderr: String },
}

pub type Result<T> = std::result::Result<T, MediaError>;
