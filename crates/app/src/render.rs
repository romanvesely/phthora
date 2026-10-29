//! Offline rendering of an effect chain to a file (export and command line).

use crate::pipeline::InputVideo;
use phthora_audio::{AudioSettings, AudioTrack, FrameLevels, MidiValues};
use phthora_core::modulator::LiveInputs;
use phthora_core::{EffectInstance, OutputMode, Playback, TimingSettings};
use phthora_gpu::{Engine, GpuContext, Readback};
use phthora_media::{AudioSource, Backend, EncodeSettings, Encoder, Format, MediaKind, Quality};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ExportJob {
    pub input: Option<PathBuf>,
    pub playback: Playback,
    pub effects: Vec<EffectInstance>,
    pub timing: TimingSettings,
    pub width: u32,
    pub height: u32,
    /// straight RGB + opacity
    pub background: [f32; 4],
    pub flip_x: bool,
    pub format: Format,
    pub quality: Quality,
    pub backend: Backend,
    pub audio: Option<AudioSource>,
    /// analysed track for audio modulators (its used part set)
    pub audio_track: Option<AudioTrack>,
    pub audio_settings: AudioSettings,
    /// MIDI values at the start of the export (held for every frame)
    pub midi: MidiValues,
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

/// Frame rate a format can store: the wanted one, or the format's maximum (GIF).
pub fn format_fps(format: Format, fps: f64) -> f64 {
    format.max_fps().map_or(fps, |m| fps.min(m))
}

/// Timing actually used for a job (stills are a single frame, GIF has a frame rate limit).
pub fn job_timing(job: &ExportJob) -> TimingSettings {
    let mut t = job.timing;
    if job.format.is_still() {
        t.mode = OutputMode::Image;
    }
    t.fps = format_fps(job.format, t.fps);
    t
}

/// Render every frame of the job and hand it to `sink` (top-down, straight RGBA8).
/// `sink` returns false to stop.
pub fn render_frames(ctx: &GpuContext, job: &ExportJob, progress: &Progress, mut sink: impl FnMut(Vec<u8>) -> Result<(), String>) -> Result<(), String> {
    let (w, h) = (job.width, job.height);
    let mut engine = Engine::new(ctx, w, h);
    crate::video::install(&mut engine.assets, false);
    engine.flip_x = job.flip_x;
    engine.background = job.background;
    if !job.format.supports_alpha() {
        engine.background[3] = 1.0;
    }
    let timing = job_timing(job);
    let frames = if timing.mode == OutputMode::Image { 1 } else { timing.frame_count().max(1) };
    progress.total.store(frames, Ordering::Relaxed);
    progress.done.store(0, Ordering::Relaxed);

    let mut video = None;
    if let Some(p) = &job.input {
        let info = phthora_media::probe(p).map_err(|e| e.to_string())?;
        if info.kind == MediaKind::Image {
            let img = phthora_media::load_image(p, 4096).map_err(|e| e.to_string())?;
            engine.set_source_rgba(img.width, img.height, &img.data);
        } else {
            video = Some(InputVideo::open(p, &info, (w, h), job.playback));
        }
    }

    let mut levels = FrameLevels::default();
    let mut readback = Readback::new(ctx, &engine.lib, w, h);
    let mut done = 0u64;
    let mut emit = |px: Vec<u8>, done: &mut u64| -> Result<(), String> {
        sink(px)?;
        *done += 1;
        progress.done.store(*done, Ordering::Relaxed);
        Ok(())
    };
    for i in 0..frames {
        if progress.cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        if let Some(v) = &video {
            match v.frame(i, timing.fps, frames, true) {
                Some((_, f)) => engine.set_source_rgba(f.width, f.height, &f.data),
                None => return Err(v.reader.error().unwrap_or_else(|| "input ended".into())),
            }
        }
        let ft = timing.at_frame(i);
        let mut live = LiveInputs::default();
        let lv = levels.at(i, timing.fps, job.audio_track.as_ref(), &job.audio_settings);
        for (k, v) in lv.bands.iter().take(live.audio.len()).enumerate() {
            live.audio[k] = *v;
        }
        engine.audio_spectrum.clone_from(&lv.spectrum);
        let resolved: Vec<_> = job.effects.iter().filter(|e| e.enabled).map(|e| e.resolve(&ft, &live, &job.midi)).collect();
        engine.render(&resolved, &ft);
        if let Some(px) = readback.push(engine.output(), engine.blank()) {
            emit(px, &mut done)?;
        }
    }
    while let Some(px) = readback.pop() {
        emit(px, &mut done)?;
    }
    Ok(())
}

/// Render and encode the job. A failed hardware encode is retried in software.
pub fn run(ctx: &GpuContext, job: &ExportJob, progress: &Progress) -> Result<PathBuf, String> {
    match encode(ctx, job, progress) {
        Err(e) if job.backend == Backend::Vaapi && e != "cancelled" => {
            eprintln!("VAAPI encode failed, retrying in software: {}", e.lines().last().unwrap_or(&e));
            encode(ctx, &ExportJob { backend: Backend::Software, ..job.clone() }, progress)
        }
        r => r,
    }
}

fn encode(ctx: &GpuContext, job: &ExportJob, progress: &Progress) -> Result<PathBuf, String> {
    let timing = job_timing(job);
    let mut settings = EncodeSettings::new(job.out.clone(), job.format, job.width, job.height, timing.fps, timing.duration);
    settings.quality = job.quality;
    settings.backend = if job.format == Format::Mp4 { job.backend } else { Backend::Software };
    settings.audio = if job.format.supports_audio() { job.audio.clone() } else { None };
    let mut enc = Encoder::start(settings).map_err(|e| e.to_string())?;
    // encode on its own thread so ffmpeg works while the GPU renders the next frames
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(2);
    let writer = std::thread::spawn(move || -> Result<Encoder, String> {
        for px in rx {
            enc.push_frame(&px).map_err(|e| e.to_string())?;
        }
        Ok(enc)
    });
    let rendered = render_frames(ctx, job, progress, |px| tx.send(px).map_err(|_| "encoder stopped".to_string()));
    drop(tx);
    let enc = writer.join().map_err(|_| "encoder thread crashed".to_string());
    match (rendered, enc) {
        (Ok(()), Ok(Ok(enc))) => enc.finish().map_err(|e| e.to_string()),
        // the encoder error explains a failed send better than "encoder stopped"
        (_, Ok(Err(e))) | (_, Err(e)) => Err(e),
        (Err(e), Ok(Ok(enc))) => {
            enc.cancel();
            Err(e)
        }
    }
}
