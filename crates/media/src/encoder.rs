//! Encoder: raw RGBA frames -> ffmpeg stdin -> MP4 / WebM / GIF / WebP / APNG / PNG sequence,
//! or a single still (PNG / JPG / WebP).

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
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
    /// Animated WebP.
    Webp,
    /// Animated PNG.
    Apng,
    /// One PNG file per frame, see [`EncodeSettings::path`].
    PngSeq,
    /// Single PNG still.
    Png,
    /// Single JPEG still.
    Jpg,
    /// Single WebP still.
    WebpStill,
}

impl Format {
    /// Formats for a single image.
    pub const STILLS: [Format; 3] = [Format::Png, Format::Jpg, Format::WebpStill];
    /// Formats for moving pictures.
    pub const ANIMATED: [Format; 6] = [Format::Mp4, Format::WebM, Format::Gif, Format::Webp, Format::Apng, Format::PngSeq];

    /// File extension; empty for [`Format::PngSeq`], which is a folder.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Mp4 => "mp4",
            Format::WebM => "webm",
            Format::Gif => "gif",
            Format::Webp | Format::WebpStill => "webp",
            Format::Apng => "apng",
            Format::PngSeq => "",
            Format::Png => "png",
            Format::Jpg => "jpg",
        }
    }

    /// Short name for buttons.
    pub fn label(self) -> &'static str {
        match self {
            Format::Mp4 => "MP4",
            Format::WebM => "WebM",
            Format::Gif => "GIF",
            Format::Webp | Format::WebpStill => "WebP",
            Format::Apng => "APNG",
            Format::PngSeq => "PNG sequence",
            Format::Png => "PNG",
            Format::Jpg => "JPG",
        }
    }

    /// Stable name for settings files.
    pub fn id(self) -> &'static str {
        match self {
            Format::Mp4 => "mp4",
            Format::WebM => "webm",
            Format::Gif => "gif",
            Format::Webp => "webp",
            Format::Apng => "apng",
            Format::PngSeq => "png_seq",
            Format::Png => "png",
            Format::Jpg => "jpg",
            Format::WebpStill => "webp_still",
        }
    }

    pub fn from_id(id: &str) -> Option<Format> {
        Self::STILLS.iter().chain(Self::ANIMATED.iter()).copied().find(|f| f.id() == id)
    }

    /// Format for an output file name, e.g. from the command line. A name with a `%05d`
    /// style number is a PNG sequence; `.webp` is a still unless `animated_webp`.
    pub fn from_path(path: &Path, animated_webp: bool) -> Option<Format> {
        let name = path.file_name()?.to_string_lossy().to_lowercase();
        if name.contains('%') {
            return name.ends_with(".png").then_some(Format::PngSeq);
        }
        Some(match path.extension()?.to_string_lossy().to_lowercase().as_str() {
            "mp4" => Format::Mp4,
            "webm" => Format::WebM,
            "gif" => Format::Gif,
            "apng" => Format::Apng,
            "png" => Format::Png,
            "jpg" | "jpeg" => Format::Jpg,
            "webp" if animated_webp => Format::Webp,
            "webp" => Format::WebpStill,
            _ => return None,
        })
    }

    /// Single image rather than a moving picture.
    pub fn is_still(self) -> bool {
        matches!(self, Format::Png | Format::Jpg | Format::WebpStill)
    }

    /// Whether the format can carry an audio track.
    pub fn supports_audio(self) -> bool {
        matches!(self, Format::Mp4 | Format::WebM)
    }

    /// Whether the format keeps the alpha channel.
    pub fn supports_alpha(self) -> bool {
        !matches!(self, Format::Mp4 | Format::Jpg)
    }

    /// Whether [`Quality`] changes the output (lossless formats ignore it).
    pub fn uses_quality(self) -> bool {
        !matches!(self, Format::Apng | Format::PngSeq | Format::Png)
    }

    /// Highest frame rate the format can store. GIF delays are whole hundredths of a second.
    pub fn max_fps(self) -> Option<f64> {
        (self == Format::Gif).then_some(50.0)
    }

    /// ffmpeg encoder this format needs (the software one for MP4).
    pub fn encoder_name(self) -> &'static str {
        match self {
            Format::Mp4 => "libx264",
            Format::WebM => "libvpx-vp9",
            Format::Gif => "gif",
            Format::Webp => "libwebp_anim",
            Format::WebpStill => "libwebp",
            Format::Apng => "apng",
            Format::PngSeq | Format::Png => "png",
            Format::Jpg => "mjpeg",
        }
    }
}

/// Encoder names of the installed ffmpeg, read once. `None` when ffmpeg cannot be run.
fn ffmpeg_encoders() -> Option<&'static HashSet<String>> {
    static ENC: std::sync::OnceLock<Option<HashSet<String>>> = std::sync::OnceLock::new();
    ENC.get_or_init(|| {
        let out = ffmpeg_cmd().arg("-encoders").stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        Some(parse_encoders(&String::from_utf8_lossy(&out.stdout)))
    })
    .as_ref()
}

/// Names from `ffmpeg -encoders`; lines look like ` V....D libwebp_anim   libwebp WebP image`.
fn parse_encoders(text: &str) -> HashSet<String> {
    text.lines().filter_map(|l| l.split_whitespace().nth(1)).map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_list_parsing() {
        let text = "Encoders:\n V..... = Video\n ------\n V....D apng                 APNG image\n V....D libwebp_anim         libwebp WebP image (codec webp)\n A....D aac                  AAC\n";
        let set = parse_encoders(text);
        assert!(set.contains("apng") && set.contains("libwebp_anim") && set.contains("aac"));
        assert!(!set.contains("libwebp"));
    }

    #[test]
    fn formats_from_names() {
        let f = |p: &str, anim| Format::from_path(Path::new(p), anim);
        assert_eq!(f("a.MP4", false), Some(Format::Mp4));
        assert_eq!(f("a.webm", false), Some(Format::WebM));
        assert_eq!(f("a.gif", false), Some(Format::Gif));
        assert_eq!(f("a.apng", false), Some(Format::Apng));
        assert_eq!(f("a.png", false), Some(Format::Png));
        assert_eq!(f("a.jpeg", false), Some(Format::Jpg));
        assert_eq!(f("a.webp", false), Some(Format::WebpStill));
        assert_eq!(f("a.webp", true), Some(Format::Webp));
        assert_eq!(f("out/frame-%05d.png", false), Some(Format::PngSeq));
        assert_eq!(f("frame-%05d.jpg", false), None);
        assert_eq!(f("a.txt", false), None);
        assert_eq!(f("noext", false), None);
    }

    #[test]
    fn sequence_paths() {
        let mut s = EncodeSettings::new("out/clip", Format::PngSeq, 8, 8, 10.0, 1.0);
        assert_eq!(output_path(&s), PathBuf::from("out/clip/clip-%05d.png"));
        s.path = "x/f_%03d.png".into();
        assert_eq!(output_path(&s), PathBuf::from("x/f_%03d.png"));
    }
}

/// The ffmpeg encoder `format` needs, if this ffmpeg build lacks it.
pub fn missing_encoder(format: Format) -> Option<&'static str> {
    let name = format.encoder_name();
    match ffmpeg_encoders() {
        Some(set) if !set.contains(name) => Some(name),
        _ => None,
    }
}

/// True if ffmpeg can encode `format` (software path).
pub fn format_available(format: Format) -> bool {
    crate::ffmpeg_available() && missing_encoder(format).is_none()
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

    /// libwebp quality factor (0..100) for lossy WebP.
    pub fn webp_quality(self) -> u32 {
        match self {
            Quality::Ultra => 95,
            Quality::High => 85,
            Quality::Medium => 70,
            Quality::Low => 50,
        }
    }

    /// MJPEG `-q:v` scale (2 best .. 31 worst).
    pub fn jpeg_qscale(self) -> u32 {
        match self {
            Quality::Ultra => 2,
            Quality::High => 4,
            Quality::Medium => 8,
            Quality::Low => 14,
        }
    }

    /// GIF palette size.
    pub fn gif_colors(self) -> u32 {
        match self {
            Quality::Ultra | Quality::High => 256,
            Quality::Medium => 128,
            Quality::Low => 64,
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
    /// Output file. For [`Format::PngSeq`]: a pattern like `dir/name-%05d.png`, or a folder
    /// (frames go inside as `<folder name>-%05d.png`; created if missing).
    pub path: PathBuf,
    pub format: Format,
    pub width: u32,
    pub height: u32,
    /// GIF takes at most 50 (see [`Format::max_fps`]).
    pub fps: f64,
    pub quality: Quality,
    /// Only used for MP4 and WebM.
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
    /// frame file pattern of a PNG sequence
    seq: Option<PathBuf>,
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
        // One palette from the statistics of all frames, error-diffusion dither (no visible
        // pattern), unchanged areas are not dithered again. Measured against the other
        // recipes on effect clips: best similarity per byte, see the encoder tests.
        (Format::Gif, _) => format!(
            "[0:v]split[s0][s1];[s0]palettegen=stats_mode=full:max_colors={}[p];[s1][p]paletteuse=dither=floyd_steinberg:diff_mode=rectangle[v]",
            s.quality.gif_colors()
        ),
        (Format::Jpg, _) => format!("[0:v]format={}[v]", if s.quality == Quality::Ultra { "yuvj444p" } else { "yuvj420p" }),
        (Format::Webp | Format::WebpStill | Format::Apng | Format::PngSeq | Format::Png, _) => "[0:v]format=rgba[v]".to_string(),
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
        Format::Webp => push(&["-c:v", "libwebp_anim", "-quality", &s.quality.webp_quality().to_string(), "-compression_level", "4", "-loop", "0", "-r", &fps]),
        Format::Apng => push(&["-c:v", "apng", "-plays", "0", "-f", "apng", "-r", &fps]),
        Format::PngSeq => push(&["-c:v", "png", "-start_number", "1", "-r", &fps]),
        Format::Png => push(&["-frames:v", "1", "-update", "1"]),
        Format::Jpg => push(&["-c:v", "mjpeg", "-q:v", &s.quality.jpeg_qscale().to_string(), "-frames:v", "1", "-update", "1"]),
        Format::WebpStill => push(&["-c:v", "libwebp", "-quality", &s.quality.webp_quality().to_string(), "-frames:v", "1"]),
    }
    if with_audio {
        push(&["-shortest"]);
    }
    if s.deterministic {
        push(&["-threads", "1", "-fflags", "+bitexact", "-flags:v", "+bitexact", "-flags:a", "+bitexact", "-map_metadata", "-1"]);
    }
    a.push(output_path(s).to_string_lossy().into_owned());
    a
}

/// File (or `%05d` pattern) ffmpeg writes to.
fn output_path(s: &EncodeSettings) -> PathBuf {
    if s.format != Format::PngSeq || s.path.to_string_lossy().contains('%') {
        return s.path.clone();
    }
    let name = s.path.file_name().map(|n| n.to_string_lossy().into_owned()).filter(|n| !n.is_empty()).unwrap_or_else(|| "frame".into());
    s.path.join(format!("{name}-%05d.png"))
}

/// Delete the files a PNG sequence pattern covers (`name-00001.png` ...), then the folder if
/// that left it empty. Nothing else in the folder is touched.
fn clear_sequence(pattern: &Path) {
    let (Some(dir), Some(name)) = (pattern.parent(), pattern.file_name().map(|n| n.to_string_lossy().into_owned())) else { return };
    let dir = if dir.as_os_str().is_empty() { Path::new(".") } else { dir };
    // `%05d` -> prefix and suffix around a run of digits
    let Some(pct) = name.find('%') else { return };
    let Some(d) = name[pct..].find('d') else { return };
    let (prefix, suffix) = (&name[..pct], &name[pct + d + 1..]);
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let f = e.file_name().to_string_lossy().into_owned();
        let digits = f.strip_prefix(prefix).and_then(|r| r.strip_suffix(suffix));
        if digits.is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    let _ = std::fs::remove_dir(dir);
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
        if let Some(max) = s.format.max_fps()
            && s.fps > max + 1e-6 {
                return bad(&format!("{} takes at most {max} fps", s.format.label()));
            }
        // the VAAPI path does not use libx264
        if !(s.format == Format::Mp4 && s.backend == Backend::Vaapi)
            && let Some(enc) = missing_encoder(s.format) {
                return bad(&format!("this ffmpeg has no {} encoder ({enc}), so {} cannot be written", s.format.label(), s.format.label()));
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
        let out = output_path(s);
        let seq = (s.format == Format::PngSeq).then(|| out.clone());
        if let Some(p) = &seq {
            // frames of an earlier export must not mix with the new ones
            clear_sequence(p);
        }
        if let Some(dir) = out.parent()
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
            seq,
            frame_len: settings.width as usize * settings.height as usize * 4,
            max_frames: settings.format.is_still().then_some(1),
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
            return Err(MediaError::InvalidInput("a still image takes exactly one frame".into()));
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

    /// Close the input, wait for ffmpeg and return the output path (the folder or pattern of
    /// a PNG sequence, as given in the settings).
    pub fn finish(mut self) -> Result<PathBuf> {
        drop(self.stdin.take());
        let status = self.child.take().map(|mut c| c.wait());
        let stderr = self.tail.take().map(|t| t.finish()).unwrap_or_default();
        match status {
            Some(Ok(st)) if st.success() => Ok(self.path.clone()),
            other => {
                self.remove_output();
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
        self.remove_output();
    }

    /// Delete what was written so far.
    fn remove_output(&self) {
        match &self.seq {
            Some(pattern) => clear_sequence(pattern),
            None => {
                let _ = std::fs::remove_file(&self.path);
            }
        }
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
