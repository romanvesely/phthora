//! Media I/O for phthora: ffmpeg/ffprobe wrapped as child processes.
//!
//! - [`probe`] / [`count_frames`]: inspect files
//! - [`load_image`]: still image to RGBA
//! - [`Decoder`]: streaming frames from video/GIF (preview and export)
//! - [`Encoder`]: frames to MP4 / WebM / GIF / PNG, optional audio mux

mod decoder;
mod encoder;
mod error;
mod image;
mod name;
mod probe;
mod size;
mod util;

pub use decoder::{DecodeOptions, Decoder, Frame};
pub use encoder::{AudioSource, Backend, EncodeSettings, Encoder, Format, Quality, vaapi_available};
pub use error::{MediaError, Result};
pub use image::{RgbaImage, load_image};
pub use name::{default_export_name, export_name_at};
pub use probe::{MediaInfo, MediaKind, count_frames, probe};
pub use size::fit_size;
pub use util::ffmpeg_available;
