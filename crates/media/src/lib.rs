//! Media I/O for phthora: ffmpeg/ffprobe wrapped as child processes.
//!
//! - [`probe`] / [`count_frames`]: inspect files
//! - [`load_image`]: still image to RGBA
//! - [`Decoder`]: streaming frames from video/GIF (preview and export)
//! - [`Encoder`]: frames to MP4 / WebM / GIF / WebP / APNG / PNG sequence or one still
//!   (PNG / JPG / WebP), optional audio mux
//! - [`LiveSource`] / [`VirtualCam`]: cameras in, virtual camera out (V4L2)

mod audio;
mod clip;
mod decoder;
mod encoder;
mod error;
mod image;
mod live;
mod name;
mod probe;
mod size;
mod util;

pub use audio::decode_audio;
pub use clip::ClipReader;
pub use decoder::{DecodeOptions, Decoder, Frame};
pub use encoder::{AudioSource, Backend, EncodeSettings, Encoder, Format, Quality, format_available, missing_encoder, vaapi_available};
pub use error::{MediaError, Result};
pub use image::{RgbaImage, load_image};
pub use live::{LiveSource, Record, VideoDevice, VirtualCam, list_devices};
pub use name::{default_export_name, export_name_at, unique_export_path, unique_export_path_at};
pub use probe::{MediaInfo, MediaKind, count_frames, probe};
pub use size::fit_size;
pub use util::ffmpeg_available;
