//! Offline rendering of an effect chain to a file (export and command line).

use phthora_core::modulator::LiveInputs;
use phthora_core::{EffectInstance, OutputMode, TimingSettings};
use phthora_gpu::{Engine, GpuContext};
use phthora_media::{AudioSource, DecodeOptions, Decoder, EncodeSettings, Encoder, Format, MediaKind, Quality};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ExportJob {
    pub input: Option<PathBuf>,
    pub effects: Vec<EffectInstance>,
    pub timing: TimingSettings,
    pub width: u32,
    pub height: u32,
    /// straight RGB + opacity
    pub background: [f32; 4],
    pub flip_x: bool,
    pub format: Format,
    pub quality: Quality,
    pub audio: Option<AudioSource>,
    pub out: PathBuf,
}

#[derive(Clone, Default)]
pub struct Progress {
    pub done: Arc<AtomicU64>,
    pub total: Arc<AtomicU64>,
    pub cancel: Arc<AtomicBool>,
}

impl Progress {
    pub fn fraction(&self) -> f32 {
        let t = self.total.load(Ordering::Relaxed).max(1);
        self.done.load(Ordering::Relaxed) as f32 / t as f32
    }
}

/// Convert premultiplied to straight alpha in place.
pub fn unpremultiply(px: &mut [u8]) {
    for p in px.as_chunks_mut::<4>().0 {
        let a = p[3] as u32;
        if a != 0 && a != 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

enum Input {
    None,
    Still,
    Video(Decoder),
}

pub fn run(ctx: &GpuContext, job: &ExportJob, progress: &Progress) -> Result<PathBuf, String> {
    let (w, h) = (job.width, job.height);
    let mut engine = Engine::new(ctx, w, h);
    crate::video::install(&mut engine.assets);
    engine.flip_x = job.flip_x;
    engine.background = job.background;
    if !job.format.supports_alpha() {
        engine.background[3] = 1.0;
    }
    let single = job.format == Format::Png;
    let timing = if single { TimingSettings { mode: OutputMode::Image, ..job.timing } } else { job.timing };
    let frames = if single { 1 } else { timing.frame_count().max(1) };
    progress.total.store(frames, Ordering::Relaxed);
    progress.done.store(0, Ordering::Relaxed);

    let mut input = Input::None;
    if let Some(p) = &job.input {
        let info = phthora_media::probe(p).map_err(|e| e.to_string())?;
        if info.kind == MediaKind::Image {
            let img = phthora_media::load_image(p, 4096).map_err(|e| e.to_string())?;
            engine.set_source_rgba(img.width, img.height, &img.data);
            input = Input::Still;
        } else {
            let (sw, sh) = phthora_media::fit_size(info.width, info.height, w.max(h));
            let opts = DecodeOptions { fps: Some(timing.fps), looped: true, ..Default::default() };
            input = Input::Video(Decoder::open(p, &info, sw, sh, opts).map_err(|e| e.to_string())?);
        }
    }

    let mut settings = EncodeSettings::new(job.out.clone(), job.format, w, h, timing.fps, timing.duration);
    settings.quality = job.quality;
    settings.audio = if job.format.supports_audio() { job.audio.clone() } else { None };
    let mut enc = Encoder::start(settings).map_err(|e| e.to_string())?;
    let live = LiveInputs::default();
    let midi = HashMap::new();

    for i in 0..frames {
        if progress.cancel.load(Ordering::Relaxed) {
            enc.cancel();
            return Err("cancelled".into());
        }
        if let Input::Video(dec) = &mut input {
            match dec.next_frame() {
                Some(f) => engine.set_source_rgba(f.width, f.height, &f.data),
                None => {
                    let err = dec.error().unwrap_or_else(|| "input ended".into());
                    enc.cancel();
                    return Err(err);
                }
            }
        }
        let ft = timing.at_frame(i);
        let resolved: Vec<_> = job.effects.iter().filter(|e| e.enabled).map(|e| e.resolve(&ft, &live, &midi)).collect();
        engine.render(&resolved, &ft);
        let mut px = engine.read_output();
        unpremultiply(&mut px);
        if let Err(e) = enc.push_frame(&px) {
            return Err(e.to_string());
        }
        progress.done.store(i + 1, Ordering::Relaxed);
    }
    enc.finish().map_err(|e| e.to_string())
}
