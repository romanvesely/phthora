//! Effect model, timing, modulators and presets. No GPU code.

pub mod chain;
pub mod clock;
pub mod history;
pub mod modulator;
pub mod noise;
pub mod params;
pub mod playback;
pub mod preset;
pub mod registry;

pub use chain::{Chain, EffectInstance, MaskSource, ResolvedEffect};
pub use clock::{FrameTime, OutputMode, TimingSettings};
pub use params::{ParamDef, ParamKind, Value};
pub use playback::{ClipTiming, Direction, Playback};
pub use registry::{EffectDesc, EffectKind};
