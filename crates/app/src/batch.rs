//! Batch export: the same effect chain over several input files, one after the other.
//! Used by `phthora render` with several inputs and by the batch window.

use crate::render::{self, ExportJob, Progress};
use phthora_audio::{AudioSettings, MidiValues};
use phthora_core::{Chain, ClipTiming, Direction, OutputMode, Playback, TimingSettings};
use phthora_gpu::GpuContext;
use phthora_media::{AudioSource, Backend, Format, MediaInfo, MediaKind, Quality};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Largest side of an output, as in the editor.
const MAX_DIM: u32 = 4096;

/// Settings that are set explicitly and then hold for every input. What is `None` follows each input:
/// its size, its length and frame rate, and a format that matches it ([`render::matched_format`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Overrides {
    pub size: Option<(u32, u32)>,
    pub duration: Option<f64>,
    pub fps: Option<f64>,
    pub format: Option<Format>,
}

impl Overrides {
    /// Whether the output of an input is a single image: a still input, unless a length or a
    /// frame rate is asked for (the same rule as for the command line's `.webp` output).
    pub fn still_mode(&self, kind: MediaKind) -> bool {
        kind == MediaKind::Image && self.duration.is_none() && self.fps.is_none()
    }

    /// Format of the output for an input. Without a set format it matches the input; a set
    /// WebP is animated for moving output and a still image otherwise.
    pub fn format_for(&self, kind: MediaKind, input: &Path) -> Format {
        let still = self.still_mode(kind);
        match self.format {
            None => render::matched_format(kind, input, still),
            Some(Format::Webp | Format::WebpStill) => if still { Format::WebpStill } else { Format::Webp },
            Some(f) => f,
        }
    }
}

/// Format named on the command line (`--format`): a file extension, or `seq` for a PNG sequence.
/// `webp` stands for both WebP formats, [`Overrides::format_for`] picks one per input.
pub fn parse_format(name: &str) -> Option<Format> {
    Some(match name.trim_start_matches('.').to_lowercase().as_str() {
        "mp4" => Format::Mp4,
        "webm" => Format::WebM,
        "gif" => Format::Gif,
        "webp" => Format::Webp,
        "apng" => Format::Apng,
        "png" => Format::Png,
        "jpg" | "jpeg" => Format::Jpg,
        "seq" | "png_seq" => Format::PngSeq,
        _ => return None,
    })
}

/// Which video encoder MP4 output asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackendPref {
    /// VAAPI where it works, else software
    Auto,
    Software,
    /// VAAPI or an error
    Vaapi,
}

/// Where the sound of an output comes from.
#[derive(Clone, Debug)]
pub enum AudioChoice {
    /// the input's own audio, if it has some and plays at normal speed forwards
    Input,
    None,
    File(PathBuf),
}

/// Everything of an export that does not depend on the input.
#[derive(Clone)]
pub struct Shared {
    pub chain: Chain,
    pub playback: Playback,
    pub bpm: Option<f64>,
    /// straight RGB + opacity
    pub background: [f32; 4],
    pub flip_x: bool,
    pub quality: Quality,
    pub backend: BackendPref,
    pub audio: AudioChoice,
    /// seconds into the audio where the exported sound starts
    pub audio_start: f64,
    pub audio_settings: AudioSettings,
    pub midi: MidiValues,
}

/// A job for one input, with a note when something was changed to make it work.
pub struct Built {
    pub job: ExportJob,
    pub note: Option<String>,
}

/// Size, timing, audio and encoder of one output. `format` and `out` are chosen by the caller;
/// `info` describes `input`.
pub fn build_job(sh: &Shared, ov: &Overrides, input: Option<&Path>, info: Option<&MediaInfo>, format: Format, out: PathBuf) -> Result<Built, String> {
    let backend = match (sh.backend, format) {
        (_, f) if f != Format::Mp4 => Backend::Software,
        (BackendPref::Software, _) => Backend::Software,
        (BackendPref::Vaapi, _) => {
            if !phthora_media::vaapi_available() {
                return Err("--backend vaapi: no working VAAPI H.264 encoder found".into());
            }
            Backend::Vaapi
        }
        (BackendPref::Auto, _) => if phthora_media::vaapi_available() { Backend::Vaapi } else { Backend::Software },
    };
    if backend == Backend::Software
        && let Some(enc) = phthora_media::missing_encoder(format) {
            return Err(format!("this ffmpeg has no {} encoder ({enc}), cannot write {}", format.label(), out.display()));
        }
    let (w, h) = ov.size.unwrap_or_else(|| info.map(|i| phthora_media::fit_size(i.width, i.height, MAX_DIM)).unwrap_or((1280, 720)));
    let video = info.filter(|i| i.kind != MediaKind::Image && i.duration > 0.0);
    let clip = video.map(|i| ClipTiming { fps: if i.fps > 0.0 { i.fps } else { 30.0 }, frames: ((i.duration * if i.fps > 0.0 { i.fps } else { 30.0 }).round() as u64).max(1) });
    let in_dur = clip.map(|c| sh.playback.natural_duration(c));
    let in_fps = video.filter(|i| i.fps > 0.0).map(|i| i.fps.round().clamp(1.0, 120.0));
    let mut timing = TimingSettings {
        mode: if format.is_still() { OutputMode::Image } else { OutputMode::Video },
        duration: ov.duration.or(in_dur).unwrap_or(4.0).clamp(0.05, 3600.0),
        fps: ov.fps.or(in_fps).unwrap_or(30.0),
        bpm: sh.bpm,
    };
    let mut note = None;
    let capped = render::format_fps(format, timing.fps);
    if capped < timing.fps {
        note = Some(format!("{} takes at most {capped} fps, using {capped}", format.label()));
        timing.fps = capped;
    }
    // the input's own audio follows the trim; reversed or sped-up input has no matching audio
    let audio = match &sh.audio {
        AudioChoice::None => None,
        AudioChoice::File(f) => Some(AudioSource { path: f.clone(), start: sh.audio_start, end: None }),
        AudioChoice::Input if sh.playback.speed == 1.0 && sh.playback.direction == Direction::Forward => video
            .filter(|i| i.has_audio)
            .and(input)
            .map(|p| AudioSource { path: p.to_path_buf(), start: sh.playback.trim_start + sh.audio_start, end: sh.playback.trim_end }),
        AudioChoice::Input => None,
    };
    let audio_track = match &audio {
        Some(a) => {
            let mut t = phthora_audio::AudioTrack::load(&a.path).map_err(|e| format!("audio: {e}"))?;
            t.start = a.start;
            t.end = a.end;
            Some(t)
        }
        None => None,
    };
    let job = ExportJob {
        input: input.map(Path::to_path_buf),
        playback: sh.playback,
        chain: sh.chain.clone(),
        timing,
        width: w & !1,
        height: h & !1,
        background: sh.background,
        flip_x: sh.flip_x,
        format,
        quality: sh.quality,
        backend,
        audio,
        audio_track,
        audio_settings: sh.audio_settings.clone(),
        midi: sh.midi.clone(),
        out,
    };
    Ok(Built { job, note })
}

/// Where a running batch is: file number, its name and the frames of that file.
#[derive(Clone, Default)]
pub struct BatchProgress {
    /// 1-based number of the file being rendered (0 before the first)
    pub index: Arc<AtomicUsize>,
    pub total: Arc<AtomicUsize>,
    pub name: Arc<Mutex<String>>,
    pub frames: Progress,
}

impl BatchProgress {
    /// Share of the whole batch: finished files plus the part of the current one.
    pub fn fraction(&self) -> f32 {
        let total = self.total.load(Ordering::Relaxed).max(1) as f32;
        let finished = self.index.load(Ordering::Relaxed).saturating_sub(1) as f32;
        ((finished + self.frames.fraction().min(1.0)) / total).min(1.0)
    }

    pub fn cancel(&self) {
        self.frames.cancel.store(true, Ordering::Relaxed);
    }
}

/// An input that did not make it.
#[derive(Clone, Debug)]
pub struct Failure {
    pub input: PathBuf,
    pub error: String,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    /// written files (a folder for a PNG sequence), in input order
    pub done: Vec<PathBuf>,
    pub failed: Vec<Failure>,
    /// stopped by the user: the inputs after the current one were not started
    pub cancelled: bool,
}

/// What happens during a batch, for a log line.
pub enum Event<'a> {
    Start { index: usize, total: usize, input: &'a Path },
    Done { index: usize, total: usize, out: &'a Path, frames: u64, secs: f32 },
    Failed { index: usize, total: usize, input: &'a Path, error: &'a str },
    Note(&'a str),
}

fn file_label(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

/// Render every input with the chain of `sh` into `out_dir`. An input that fails is reported and
/// the next one goes on; cancelling stops the batch after removing the partial output.
pub fn run_batch(ctx: &GpuContext, inputs: &[PathBuf], out_dir: &Path, sh: &Shared, ov: &Overrides, progress: &BatchProgress, mut on_event: impl FnMut(Event)) -> Report {
    let mut report = Report::default();
    let total = inputs.len();
    progress.total.store(total, Ordering::Relaxed);
    progress.index.store(0, Ordering::Relaxed);
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        let error = format!("cannot create {}: {e}", out_dir.display());
        for input in inputs {
            report.failed.push(Failure { input: input.clone(), error: error.clone() });
        }
        return report;
    }
    let mut taken = HashSet::new();
    for (i, input) in inputs.iter().enumerate() {
        let index = i + 1;
        if progress.frames.cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        progress.index.store(index, Ordering::Relaxed);
        *progress.name.lock().unwrap() = file_label(input);
        progress.frames.done.store(0, Ordering::Relaxed);
        progress.frames.total.store(1, Ordering::Relaxed);
        on_event(Event::Start { index, total, input });
        let t0 = Instant::now();
        let result = (|| {
            let info = phthora_media::probe(input).map_err(|e| e.to_string())?;
            let format = ov.format_for(info.kind, input);
            let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let out = phthora_media::unique_export_path(out_dir, &stem, format.extension(), &taken);
            taken.insert(out.clone());
            let built = build_job(sh, ov, Some(input), Some(&info), format, out)?;
            if let Some(n) = &built.note {
                on_event(Event::Note(n));
            }
            render::run(ctx, &built.job, &progress.frames)
        })();
        match result {
            Ok(path) => {
                on_event(Event::Done { index, total, out: &path, frames: progress.frames.done.load(Ordering::Relaxed), secs: t0.elapsed().as_secs_f32() });
                report.done.push(path);
            }
            Err(e) if e == "cancelled" => {
                report.cancelled = true;
                break;
            }
            Err(error) => {
                on_event(Event::Failed { index, total, input, error: &error });
                report.failed.push(Failure { input: input.clone(), error });
            }
        }
    }
    report
}

/// One line for the end of a batch: counts, and what failed.
pub fn summary(r: &Report) -> String {
    let mut s = format!("{} exported", r.done.len());
    if !r.failed.is_empty() {
        s.push_str(&format!(", {} failed", r.failed.len()));
    }
    if r.cancelled {
        s.push_str(", cancelled");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(name: &str) -> (MediaKind, PathBuf) {
        let k = if name.ends_with(".png") || name.ends_with(".jpg") { MediaKind::Image } else if name.ends_with(".gif") { MediaKind::Gif } else { MediaKind::Video };
        (k, PathBuf::from(name))
    }

    #[test]
    fn format_follows_the_input_unless_set() {
        let ov = Overrides::default();
        let f = |name: &str, ov: &Overrides| {
            let (k, p) = kind(name);
            ov.format_for(k, &p)
        };
        assert_eq!(f("a.mp4", &ov), Format::Mp4);
        assert_eq!(f("a.mkv", &ov), Format::Mp4);
        assert_eq!(f("a.gif", &ov), Format::Gif);
        assert_eq!(f("a.png", &ov), Format::Png);
        assert_eq!(f("a.jpg", &ov), Format::Jpg);
        // a length or frame rate turns a still input into a moving picture
        assert_eq!(f("a.png", &Overrides { duration: Some(2.0), ..ov }), Format::Mp4);
        assert_eq!(f("a.jpg", &Overrides { fps: Some(10.0), ..ov }), Format::Mp4);
        // a set format holds for everything; WebP is moving or still as the output is
        let webp = Overrides { format: Some(Format::Webp), ..ov };
        assert_eq!(f("a.mp4", &webp), Format::Webp);
        assert_eq!(f("a.png", &webp), Format::WebpStill);
        assert_eq!(f("a.png", &Overrides { duration: Some(1.0), ..webp }), Format::Webp);
        let gif = Overrides { format: Some(Format::Gif), ..ov };
        assert_eq!(f("a.png", &gif), Format::Gif);
        assert_eq!(f("a.mp4", &Overrides { format: Some(Format::Png), ..ov }), Format::Png);
    }

    #[test]
    fn format_names() {
        assert_eq!(parse_format("mp4"), Some(Format::Mp4));
        assert_eq!(parse_format(".WebM"), Some(Format::WebM));
        assert_eq!(parse_format("jpeg"), Some(Format::Jpg));
        assert_eq!(parse_format("seq"), Some(Format::PngSeq));
        assert_eq!(parse_format("webp"), Some(Format::Webp));
        assert_eq!(parse_format("mov"), None);
    }

    fn shared() -> Shared {
        Shared {
            chain: Chain::default(),
            playback: Playback::default(),
            bpm: None,
            background: [0.0, 0.0, 0.0, 1.0],
            flip_x: false,
            quality: Quality::High,
            backend: BackendPref::Software,
            audio: AudioChoice::Input,
            audio_start: 0.0,
            audio_settings: AudioSettings::default(),
            midi: MidiValues::new(),
        }
    }

    fn info(kind: MediaKind, w: u32, h: u32, secs: f64, fps: f64) -> MediaInfo {
        MediaInfo { kind, width: w, height: h, rotation: 0, duration: secs, fps, frame_count: 0, has_audio: false, codec: String::new(), vfr: false }
    }

    fn video_info(w: u32, h: u32, secs: f64, fps: f64) -> MediaInfo {
        info(MediaKind::Video, w, h, secs, fps)
    }

    #[test]
    fn size_and_timing_come_from_each_input_unless_overridden() {
        let sh = shared();
        let none = Overrides::default();
        let job = |ov: &Overrides, info: &MediaInfo, format| build_job(&sh, ov, None, Some(info), format, PathBuf::from("o")).unwrap().job;
        let a = video_info(1920, 1080, 4.0, 25.0);
        let b = video_info(640, 360, 2.0, 30.0);
        let ja = job(&none, &a, Format::Mp4);
        let jb = job(&none, &b, Format::Mp4);
        assert_eq!((ja.width, ja.height, ja.timing.fps, ja.timing.duration), (1920, 1080, 25.0, 4.0));
        assert_eq!((jb.width, jb.height, jb.timing.fps, jb.timing.duration), (640, 360, 30.0, 2.0));
        // an override holds for all, odd sizes become even
        let ov = Overrides { size: Some((321, 181)), duration: Some(1.5), fps: Some(12.0), format: None };
        for info in [&a, &b] {
            let j = job(&ov, info, Format::Mp4);
            assert_eq!((j.width, j.height, j.timing.fps, j.timing.duration), (320, 180, 12.0, 1.5));
        }
        // a single override leaves the rest to the input
        let only_fps = job(&Overrides { fps: Some(10.0), ..none }, &b, Format::Mp4);
        assert_eq!((only_fps.width, only_fps.timing.fps, only_fps.timing.duration), (640, 10.0, 2.0));
        // GIF is capped, a still is one image
        assert_eq!(job(&Overrides { fps: Some(60.0), ..none }, &b, Format::Gif).timing.fps, 50.0);
        assert_eq!(job(&none, &b, Format::Png).timing.mode, OutputMode::Image);
        // a still input without length: 4 s of video when it is exported as video
        let still = info(MediaKind::Image, 100, 60, 0.0, 0.0);
        assert_eq!(job(&none, &still, Format::Mp4).timing.duration, 4.0);
    }

    #[test]
    fn input_audio_only_where_it_matches() {
        let mut sh = shared();
        let mut info = video_info(64, 64, 2.0, 10.0);
        info.has_audio = true;
        let p = Path::new("/nonexistent/in.mp4");
        // the audio file cannot be analysed here: the error names it, so we know it was asked for
        let e = build_job(&sh, &Overrides::default(), Some(p), Some(&info), Format::Mp4, PathBuf::new()).err();
        assert!(e.is_some_and(|e| e.starts_with("audio:")));
        sh.playback.speed = 2.0;
        assert!(build_job(&sh, &Overrides::default(), Some(p), Some(&info), Format::Mp4, PathBuf::new()).unwrap().job.audio.is_none());
        sh.playback.speed = 1.0;
        sh.audio = AudioChoice::None;
        assert!(build_job(&sh, &Overrides::default(), Some(p), Some(&info), Format::Mp4, PathBuf::new()).unwrap().job.audio.is_none());
    }
}
