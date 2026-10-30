//! Audio for phthora: spectrum analysis, band levels for audio-reactive modulation,
//! analysed tracks for preview and export, playback, live input and MIDI.

pub mod analyzer;
pub mod bands;
pub mod input;
pub mod midi;
pub mod onset;
pub mod player;
pub mod track;

pub use analyzer::{Analyzer, BINS, FFT_SIZE, HOP, SAMPLE_RATE};
pub use input::MicInput;
pub use onset::{DrumHits, Hit, LiveDrums};
pub use midi::{MidiIn, MidiValues};
pub use player::{Player, Program};
pub use bands::{AudioSettings, BandDef, Follower, Levels};
pub use track::{AudioTrack, FrameLevels};
