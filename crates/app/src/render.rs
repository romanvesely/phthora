//! Offline rendering of an effect chain to a file (export and command line).

use crate::pipeline::InputVideo;
use phthora_audio::{AudioSettings, AudioTrack, FrameLevels, MidiValues};
use phthora_core::modulator::LiveInputs;
use phthora_core::{EffectInstance, OutputMode, Playback, TimingSettings};
use phthora_gpu::{Engine, GpuContext, Readback};
use phthora_media::{AudioSource, Backend, EncodeSettings, Encoder, Format, MediaKind, Quality};
use std::path::{Path, PathBuf};
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

/// Export format that matches an input file: its own container where there is an export
/// format for it (GIF, WebM, WebP, PNG/APNG, JPG), else MP4 for moving and PNG for still
/// inputs. `still_mode` is the output mode: a still input exported as video gets MP4.
pub fn matched_format(kind: MediaKind, input: &Path, still_mode: bool) -> Format {
    let moving = kind != MediaKind::Image;
    let ext = input.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let same = match (ext.as_str(), moving) {
        ("gif", _) => Format::Gif,
        ("webm", _) => Format::WebM,
        ("webp", true) => Format::Webp,
        ("webp", false) => Format::WebpStill,
        ("png" | "apng", true) => Format::Apng,
        ("png", false) => Format::Png,
        ("jpg" | "jpeg", _) => Format::Jpg,
        (_, true) => Format::Mp4,
        (_, false) => Format::Png,
    };
    match (same.is_still(), still_mode) {
        (a, b) if a == b => same,
        (_, true) => Format::Png,
        (_, false) => Format::Mp4,
    }
}

/// GIFs beyond this are big: wider than 800 px, longer than 10 s or faster than 30 fps.
pub fn gif_is_large(width: u32, duration: f64, fps: f64) -> bool {
    width > 800 || duration > 10.0 || fps > 30.0
}

/// Rough size and memory of a GIF export, for the warning in the export window.
pub struct GifEstimate {
    /// likely range in MB: smooth gradients to film grain
    pub size_mb: (f64, f64),
    /// ffmpeg keeps every frame while it builds the palette
    pub memory_mb: f64,
}

/// Bytes per pixel and frame measured on effect clips (flat test pattern 0.05, smooth gradients 0.13, a video
/// with grain 0.19, VHS noise 1.1 at 256 colors; fewer colors give smaller files).
pub fn gif_estimate(width: u32, height: u32, fps: f64, duration: f64, quality: Quality) -> GifEstimate {
    let px = width as f64 * height as f64 * (format_fps(Format::Gif, fps) * duration).round().max(1.0);
    let colors = match quality {
        Quality::Ultra | Quality::High => 1.0,
        Quality::Medium => 0.85,
        Quality::Low => 0.7,
    };
    GifEstimate { size_mb: (px * 0.05 * colors / 1e6, px * 1.15 * colors / 1e6), memory_mb: px * 4.0 * 1.2 / 1e6 + 70.0 }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_the_input_format() {
        let f = |kind, name: &str, still| matched_format(kind, Path::new(name), still);
        assert_eq!(f(MediaKind::Video, "a.mp4", false), Format::Mp4);
        assert_eq!(f(MediaKind::Video, "a.MKV", false), Format::Mp4);
        assert_eq!(f(MediaKind::Video, "a.webm", false), Format::WebM);
        assert_eq!(f(MediaKind::Gif, "a.gif", false), Format::Gif);
        assert_eq!(f(MediaKind::Video, "a.webp", false), Format::Webp);
        assert_eq!(f(MediaKind::Image, "a.png", true), Format::Png);
        assert_eq!(f(MediaKind::Image, "a.JPG", true), Format::Jpg);
        assert_eq!(f(MediaKind::Image, "a.webp", true), Format::WebpStill);
        assert_eq!(f(MediaKind::Image, "a.bmp", true), Format::Png);
        // output mode wins over the input kind
        assert_eq!(f(MediaKind::Image, "a.jpg", false), Format::Mp4);
        assert_eq!(f(MediaKind::Video, "a.mp4", true), Format::Png);
    }

    #[test]
    fn gif_warning_thresholds() {
        assert!(!gif_is_large(800, 10.0, 30.0));
        assert!(gif_is_large(801, 4.0, 25.0) && gif_is_large(640, 10.5, 25.0) && gif_is_large(640, 4.0, 31.0));
    }

    #[test]
    fn gif_estimate_matches_measurements() {
        // measured: 1280x720, 100 frames, flat test pattern: 4.6 MB, 476 MB of memory
        let e = gif_estimate(1280, 720, 25.0, 4.0, Quality::High);
        assert!(e.size_mb.0 < 4.6 * 1.1 && e.size_mb.1 > 4.6, "{:?}", e.size_mb);
        assert!((e.memory_mb - 476.0).abs() < 80.0, "{}", e.memory_mb);
        // 60 fps counts as 50 (the GIF limit), fewer colors are smaller
        let a = gif_estimate(640, 360, 60.0, 2.0, Quality::High);
        let b = gif_estimate(640, 360, 50.0, 2.0, Quality::High);
        assert_eq!(a.size_mb, b.size_mb);
        assert!(gif_estimate(640, 360, 25.0, 2.0, Quality::Low).size_mb.1 < gif_estimate(640, 360, 25.0, 2.0, Quality::High).size_mb.1);
    }

    #[test]
    fn job_timing_limits_gif_and_stills() {
        let mut job = ExportJob {
            input: None,
            playback: Playback::default(),
            effects: Vec::new(),
            timing: TimingSettings { fps: 60.0, ..Default::default() },
            width: 64,
            height: 64,
            background: [0.0; 4],
            flip_x: false,
            format: Format::Gif,
            quality: Quality::High,
            backend: Backend::Software,
            audio: None,
            audio_track: None,
            audio_settings: AudioSettings::default(),
            midi: MidiValues::new(),
            out: PathBuf::new(),
        };
        assert_eq!(job_timing(&job).fps, 50.0);
        job.format = Format::Webp;
        assert_eq!(job_timing(&job).fps, 60.0);
        for f in [Format::Png, Format::Jpg, Format::WebpStill] {
            job.format = f;
            assert_eq!(job_timing(&job).mode, OutputMode::Image);
        }
    }
}
