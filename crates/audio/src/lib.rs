//! Audio for phthora: spectrum analysis, band levels for audio-reactive modulation,
//! analysed tracks for preview and export, playback and live input.

pub mod analyzer;
pub mod bands;
pub mod input;
pub mod player;
pub mod track;

pub use analyzer::{Analyzer, BINS, FFT_SIZE, HOP, SAMPLE_RATE};
pub use input::MicInput;
pub use player::{Player, Program};
pub use bands::{AudioSettings, BandDef, Follower, Levels};
pub use track::{AudioTrack, FrameLevels};
