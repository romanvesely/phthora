//! Encoder: raw RGBA frames -> ffmpeg stdin -> MP4 / WebM / GIF / PNG.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{MediaError, Result};
use crate::probe::run_ffprobe;
use crate::util::{StderrTail, fps_arg, ffmpeg_cmd, require_ffmpeg, secs_arg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mp4,
    WebM,
    Gif,
    Png,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Mp4 => "mp4",
            Format::WebM => "webm",
            Format::Gif => "gif",
            Format::Png => "png",
        }
    }

    /// Whether the format can carry an audio track.
    pub fn supports_audio(self) -> bool {
        matches!(self, Format::Mp4 | Format::WebM)
    }

    /// Whether the format keeps the alpha channel.
    pub fn supports_alpha(self) -> bool {
        matches!(self, Format::WebM | Format::Gif | Format::Png)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Ultra,
    High,
    Medium,
    Low,
}

impl Quality {
    /// CRF used for both x264 and VP9.
    pub fn crf(self) -> u32 {
        match self {
            Quality::Ultra => 15,
            Quality::High => 23,
            Quality::Medium => 31,
            Quality::Low => 39,
        }
    }

    /// QP used by the VAAPI H.264 encoder (roughly matching the CRF look).
    fn vaapi_qp(self) -> u32 {
        match self {
            Quality::Ultra => 18,
            Quality::High => 24,
            Quality::Medium => 30,
            Quality::Low => 36,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// libx264 (default).
    #[default]
    Software,
    /// `h264_vaapi` on Linux, MP4 only. Opt-in; see [`vaapi_available`].
    Vaapi,
}

#[derive(Debug, Clone)]
pub struct AudioSource {
    pub path: PathBuf,
    /// Trim start in seconds.
    pub start: f64,
    /// Trim end in seconds; `None` = until the file ends. The trimmed part is
    /// looped to cover the full output length.
    pub end: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct EncodeSettings {
    pub path: PathBuf,
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub quality: Quality,
    /// Ignored for GIF and PNG.
    pub audio: Option<AudioSource>,
    /// Planned output length in seconds; used for [`Encoder::expected_frames`].
    pub duration: f64,
    pub backend: Backend,
    /// Single-threaded encode with bitexact flags: same input frames give the
    /// same file bytes. Much slower.
    pub deterministic: bool,
}

impl EncodeSettings {
    pub fn new(path: impl Into<PathBuf>, format: Format, width: u32, height: u32, fps: f64, duration: f64) -> Self {
        Self {
            path: path.into(),
            format,
            width,
            height,
            fps,
            quality: Quality::High,
            audio: None,
            duration,
            backend: Backend::Software,
            deterministic: false,
        }
    }
}

pub struct Encoder {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    tail: Option<StderrTail>,
    path: PathBuf,
    frame_len: usize,
    max_frames: Option<u64>,
    expected: u64,
    written: Arc<AtomicU64>,
}

/// Render node that can encode H.264 through VAAPI, found once by test encodes.
fn vaapi_device() -> Option<&'static str> {
    static DEV: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    DEV.get_or_init(|| {
        if !crate::ffmpeg_available() {
            return None;
        }
        let mut nodes: Vec<String> = std::fs::read_dir("/dev/dri")
            .ok()?
            .flatten()
            .map(|e| e.path().to_string_lossy().into_owned())
            .filter(|p| p.rsplit('/').next().is_some_and(|n| n.starts_with("renderD")))
            .collect();
        nodes.sort();
        nodes.into_iter().find(|dev| {
            ffmpeg_cmd()
                .args(["-vaapi_device", dev, "-f", "lavfi", "-i", "color=black:s=256x256:r=5:d=0.4"])
                .args(["-vf", "format=nv12,hwupload", "-c:v", "h264_vaapi", "-f", "null", "-"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
    })
    .as_deref()
}

/// True if an H.264 VAAPI encode works on this machine. The first call runs test
/// encodes (a fraction of a second); the result is cached.
pub fn vaapi_available() -> bool {
    vaapi_device().is_some()
}

/// RGB -> YUV conversion step. Uses BT.709 / limited range so players show the colors the app rendered.
const COLOR_SCALE: &str = "scale=out_color_matrix=bt709:out_range=tv:flags=accurate_rnd+full_chroma_int";

fn audio_chain(a: &AudioSource) -> String {
    let mut trim = format!("atrim=start={}", secs_arg(a.start));
    if let Some(e) = a.end {
        trim.push_str(&format!(":end={}", secs_arg(e)));
    }
    format!("{trim},aloop=-1:2e+09,asetpts=N/SR/TB")
}

fn build_args(s: &EncodeSettings, with_audio: bool) -> Vec<String> {
    let fps = fps_arg(s.fps);
    let mut a: Vec<String> = Vec::new();
    let mut push = |v: &[&str]| a.extend(v.iter().map(|x| x.to_string()));
    push(&["-y"]);
    if s.backend == Backend::Vaapi {
        push(&["-vaapi_device", vaapi_device().unwrap_or("/dev/dri/renderD128")]);
    }
    push(&["-f", "rawvideo", "-pix_fmt", "rgba", "-s"]);
    a.push(format!("{}x{}", s.width, s.height));
    let mut push = |v: &[&str]| a.extend(v.iter().map(|x| x.to_string()));
    push(&["-framerate", &fps, "-i", "-"]);
    if with_audio {
        let au = s.audio.as_ref().unwrap();
        push(&["-stream_loop", "-1", "-i", &au.path.to_string_lossy()]);
    }

    let vgraph = match (s.format, s.backend) {
        (Format::Mp4, Backend::Vaapi) => format!("[0:v]{COLOR_SCALE},format=nv12,hwupload[v]"),
        (Format::Mp4, _) => format!("[0:v]{COLOR_SCALE},format=yuv420p[v]"),
        (Format::WebM, _) => format!("[0:v]{COLOR_SCALE},format=yuva420p[v]"),
        (Format::Gif, _) => "[0:v]split[s0][s1];[s0]palettegen=stats_mode=diff:max_colors=256[p];[s1][p]paletteuse=dither=bayer:bayer_scale=3[v]".to_string(),
        (Format::Png, _) => "[0:v]format=rgba[v]".to_string(),
    };
    let graph = if with_audio {
        format!("{vgraph};[1:a]{}[a]", audio_chain(s.audio.as_ref().unwrap()))
    } else {
        vgraph
    };
    push(&["-filter_complex", &graph, "-map", "[v]"]);
    if with_audio {
        push(&["-map", "[a]"]);
    }

    let crf = s.quality.crf().to_string();
    let color_tags = ["-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-color_range", "tv"];
    match s.format {
        Format::Mp4 => {
            if s.backend == Backend::Vaapi {
                push(&["-c:v", "h264_vaapi", "-qp", &s.quality.vaapi_qp().to_string()]);
            } else {
                push(&["-c:v", "libx264", "-preset", "medium", "-crf", &crf]);
            }
            push(&color_tags);
            push(&["-r", &fps, "-movflags", "+faststart"]);
            if with_audio {
                push(&["-c:a", "aac", "-b:a", "192k"]);
            }
        }
        Format::WebM => {
            push(&["-c:v", "libvpx-vp9", "-crf", &crf, "-b:v", "0", "-row-mt", "1", "-tile-columns", "2"]);
            // Alpha needs auto-alt-ref off in libvpx.
            push(&["-auto-alt-ref", "0", "-deadline", "good", "-cpu-used", "3"]);
            push(&color_tags);
            push(&["-r", &fps]);
            if with_audio {
                push(&["-c:a", "libopus", "-b:a", "160k"]);
            }
        }
        Format::Gif => push(&["-r", &fps, "-loop", "0"]),
        Format::Png => push(&["-frames:v", "1", "-update", "1"]),
    }
    if with_audio {
        push(&["-shortest"]);
    }
    if s.deterministic {
        push(&["-threads", "1", "-fflags", "+bitexact", "-flags:v", "+bitexact", "-flags:a", "+bitexact", "-map_metadata", "-1"]);
    }
    a.push(s.path.to_string_lossy().into_owned());
    a
}

impl Encoder {
    /// Validate settings and start ffmpeg.
    pub fn start(settings: EncodeSettings) -> Result<Encoder> {
        require_ffmpeg()?;
        let s = &settings;
        let bad = |m: &str| Err(MediaError::InvalidInput(m.to_string()));
        if s.width == 0 || s.height == 0 {
            return bad("width and height must be non-zero");
        }
        if !(s.fps > 0.0 && s.fps.is_finite()) {
            return bad("fps must be positive");
        }
        if matches!(s.format, Format::Mp4 | Format::WebM) && (!s.width.is_multiple_of(2) || !s.height.is_multiple_of(2)) {
            return bad("width and height must be even for MP4/WebM");
        }
        if s.backend == Backend::Vaapi && s.format != Format::Mp4 {
            return bad("VAAPI backend only supports MP4");
        }
        if s.backend == Backend::Vaapi && !vaapi_available() {
            return bad("no VAAPI H.264 encoder on this machine");
        }
        let mut with_audio = false;
        if let Some(au) = &s.audio
            && s.format.supports_audio() {
                let raw = run_ffprobe(&au.path)?;
                if !raw.streams.iter().any(|st| st.codec_type.as_deref() == Some("audio")) {
                    return bad("audio source has no audio stream");
                }
                with_audio = true;
            }
        if let Some(dir) = s.path.parent()
            && !dir.as_os_str().is_empty() && !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }
        let args = build_args(s, with_audio);
        let mut child = ffmpeg_cmd()
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take();
        let tail = StderrTail::spawn(child.stderr.take().unwrap());
        Ok(Encoder {
            child: Some(child),
            stdin,
            tail: Some(tail),
            path: settings.path.clone(),
            frame_len: settings.width as usize * settings.height as usize * 4,
            max_frames: (settings.format == Format::Png).then_some(1),
            expected: (settings.duration * settings.fps).round().max(0.0) as u64,
            written: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Append one frame: `width * height * 4` bytes, top-down RGBA, straight alpha.
    /// Blocks while ffmpeg is busy (natural backpressure).
    pub fn push_frame(&mut self, rgba: &[u8]) -> Result<()> {
        if rgba.len() != self.frame_len {
            return Err(MediaError::InvalidInput(format!(
                "frame has {} bytes, expected {}",
                rgba.len(),
                self.frame_len
            )));
        }
        if self.max_frames.is_some_and(|m| self.written.load(Ordering::Relaxed) >= m) {
            return Err(MediaError::InvalidInput("PNG output takes exactly one frame".into()));
        }
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(MediaError::InvalidInput("encoder already finished".into()));
        };
        match stdin.write_all(rgba) {
            Ok(()) => {
                self.written.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(e) => {
                // ffmpeg died: report its error output instead of a bare EPIPE.
                let err = self.collect_failure(Some(e));
                self.discard();
                Err(err)
            }
        }
    }

    /// Frames handed to ffmpeg so far.
    pub fn frames_written(&self) -> u64 {
        self.written.load(Ordering::Relaxed)
    }

    /// Shared counter, e.g. to show progress from another thread.
    pub fn frames_counter(&self) -> Arc<AtomicU64> {
        self.written.clone()
    }

    /// `round(duration * fps)` from the settings.
    pub fn expected_frames(&self) -> u64 {
        self.expected
    }

    /// Close the input, wait for ffmpeg and return the output path.
    pub fn finish(mut self) -> Result<PathBuf> {
        drop(self.stdin.take());
        let status = self.child.take().map(|mut c| c.wait());
        let stderr = self.tail.take().map(|t| t.finish()).unwrap_or_default();
        match status {
            Some(Ok(st)) if st.success() => Ok(self.path.clone()),
            other => {
                let _ = std::fs::remove_file(&self.path);
                let context = match other {
                    Some(Ok(st)) => format!("exit status {st}"),
                    Some(Err(e)) => e.to_string(),
                    None => "no process".into(),
                };
                Err(MediaError::Ffmpeg { context, stderr })
            }
        }
    }

    /// Abort: kill ffmpeg and delete the partial output.
    pub fn cancel(mut self) {
        self.discard();
    }

    fn discard(&mut self) {
        drop(self.stdin.take());
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        if let Some(t) = self.tail.take() {
            let _ = t.finish();
        }
        let _ = std::fs::remove_file(&self.path);
    }

    fn collect_failure(&mut self, io: Option<std::io::Error>) -> MediaError {
        drop(self.stdin.take());
        let status = self.child.as_mut().and_then(|c| c.wait().ok());
        let stderr = self.tail.take().map(|t| t.finish()).unwrap_or_default();
        let context = match (status, io) {
            (Some(st), _) => format!("exit status {st}"),
            (None, Some(e)) => e.to_string(),
            _ => "unknown".into(),
        };
        MediaError::Ffmpeg { context, stderr }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        // Dropped without finish(): treat as cancel.
        if self.child.is_some() {
            self.discard();
        }
    }
}
