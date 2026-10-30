use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

use serde::Deserialize;

use crate::error::{MediaError, Result};
use crate::util::{require_ffmpeg, tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
    Gif,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaInfo {
    pub kind: MediaKind,
    /// Display width (after rotation).
    pub width: u32,
    /// Display height (after rotation).
    pub height: u32,
    /// Clockwise degrees (0, 90, 180, 270) the stored picture is rotated for
    /// display. ffmpeg applies this itself when decoding.
    pub rotation: i32,
    /// Seconds; 0 for still images.
    pub duration: f64,
    /// Average frame rate; 0 for still images.
    pub fps: f64,
    /// `nb_frames` if the container has it, else `round(duration * fps)`.
    /// Use [`count_frames`] for an exact value.
    pub frame_count: u64,
    pub has_audio: bool,
    pub codec: String,
    /// Average and nominal frame rate differ by more than 1 %.
    pub vfr: bool,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawProbe {
    #[serde(default)]
    pub streams: Vec<RawStream>,
    pub format: Option<RawFormat>,
    #[serde(default)]
    pub frames: Vec<RawFrame>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawFrame {
    pub side_data_list: Option<Vec<RawSideData>>,
    pub tags: Option<HashMap<String, String>>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawFormat {
    pub format_name: Option<String>,
    pub duration: Option<String>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawStream {
    pub codec_type: Option<String>,
    pub codec_name: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub avg_frame_rate: Option<String>,
    pub r_frame_rate: Option<String>,
    pub nb_frames: Option<String>,
    pub duration: Option<String>,
    pub tags: Option<HashMap<String, String>>,
    pub side_data_list: Option<Vec<RawSideData>>,
    pub disposition: Option<HashMap<String, i64>>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawSideData {
    pub rotation: Option<f64>,
}

pub(crate) fn run_ffprobe(path: &Path) -> Result<RawProbe> {
    require_ffmpeg()?;
    let out = tool("ffprobe")
        .args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
        .arg(path)
        .stdin(Stdio::null())
        .output()?;
    if !out.status.success() {
        return Err(MediaError::Probe {
            path: path.to_path_buf(),
            message: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    serde_json::from_slice(&out.stdout).map_err(|e| MediaError::Probe {
        path: path.to_path_buf(),
        message: format!("bad ffprobe json: {e}"),
    })
}

fn first_frame_probe(path: &Path) -> Result<RawProbe> {
    let out = tool("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-read_intervals", "%+#1", "-show_frames", "-show_entries", "frame=side_data_list:frame_tags=Orientation", "-of", "json"])
        .arg(path)
        .stdin(Stdio::null())
        .output()?;
    serde_json::from_slice(&out.stdout).map_err(|e| MediaError::Probe {
        path: path.to_path_buf(),
        message: format!("bad ffprobe json: {e}"),
    })
}

fn parse_rate(s: Option<&str>) -> f64 {
    let Some(s) = s else { return 0.0 };
    match s.split_once('/') {
        Some((n, d)) => {
            let (n, d) = (n.parse::<f64>().unwrap_or(0.0), d.parse::<f64>().unwrap_or(0.0));
            if d > 0.0 { n / d } else { 0.0 }
        }
        None => s.parse().unwrap_or(0.0),
    }
}

fn parse_secs(s: Option<&str>) -> Option<f64> {
    s.and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite() && *v >= 0.0)
}

/// Clockwise display rotation in degrees, normalized to 0/90/180/270.
fn rotation_of(s: &RawStream, frame: Option<&RawFrame>) -> i32 {
    // Display matrix: counter-clockwise angle, so negate for clockwise.
    let cw = s
        .side_data_list
        .as_ref()
        .or(frame.and_then(|f| f.side_data_list.as_ref()))
        .and_then(|l| l.iter().find_map(|d| d.rotation))
        .map(|r| -r)
        .or_else(|| {
            s.tags
                .as_ref()
                .and_then(|t| t.get("rotate"))
                .and_then(|v| v.parse::<f64>().ok())
        })
        // ffmpeg before 6.1 gives a still's EXIF orientation only as a tag ("    6"; mirrored ones keep their turn)
        .or_else(|| match frame?.tags.as_ref()?.get("Orientation")?.trim() {
            "3" | "4" => Some(180.0),
            "5" | "6" => Some(90.0),
            "7" | "8" => Some(270.0),
            _ => None,
        });
    let deg = cw.unwrap_or(0.0);
    let quarter = (deg / 90.0).round() as i32;
    (quarter.rem_euclid(4)) * 90
}

/// Inspect a media file with ffprobe.
pub fn probe(path: &Path) -> Result<MediaInfo> {
    let raw = run_ffprobe(path)?;
    let fail = |m: &str| MediaError::Probe {
        path: path.to_path_buf(),
        message: m.to_string(),
    };
    let is_pic = |s: &&RawStream| {
        s.disposition
            .as_ref()
            .and_then(|d| d.get("attached_pic"))
            .copied()
            .unwrap_or(0)
            != 0
    };
    let video = raw
        .streams
        .iter()
        .filter(|s| s.codec_type.as_deref() == Some("video"))
        .find(|s| !is_pic(s))
        .or_else(|| raw.streams.iter().find(|s| s.codec_type.as_deref() == Some("video")))
        .ok_or_else(|| fail("no video stream"))?;
    let (cw, ch) = match (video.width, video.height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => (w, h),
        _ => return Err(fail("video stream has no size")),
    };
    let codec = video.codec_name.clone().unwrap_or_default();
    let fmt_name = raw
        .format
        .as_ref()
        .and_then(|f| f.format_name.clone())
        .unwrap_or_default();
    let has_audio = raw
        .streams
        .iter()
        .any(|s| s.codec_type.as_deref() == Some("audio"));

    let kind = if codec == "gif" || codec == "apng" {
        MediaKind::Gif
    } else if fmt_name.starts_with("image2") || fmt_name.ends_with("_pipe") || fmt_name == "ico" {
        MediaKind::Image
    } else {
        MediaKind::Video
    };

    // Still images (EXIF orientation) carry their rotation on the frame, not
    // the stream; look at the first frame only.
    let frame_rot = if kind == MediaKind::Image && video.side_data_list.is_none() {
        first_frame_probe(path).ok()
    } else {
        None
    };
    let rotation = rotation_of(video, frame_rot.as_ref().and_then(|f| f.frames.first()));
    let (width, height) = if rotation == 90 || rotation == 270 { (ch, cw) } else { (cw, ch) };

    if kind == MediaKind::Image {
        return Ok(MediaInfo {
            kind,
            width,
            height,
            rotation,
            duration: 0.0,
            fps: 0.0,
            frame_count: 1,
            has_audio: false,
            codec,
            vfr: false,
        });
    }

    let avg = parse_rate(video.avg_frame_rate.as_deref());
    let nominal = parse_rate(video.r_frame_rate.as_deref());
    let fps = if avg > 0.0 { avg } else { nominal };
    let vfr = avg > 0.0 && nominal > 0.0 && ((avg - nominal).abs() / nominal) > 0.01;
    let duration = parse_secs(video.duration.as_deref())
        .or_else(|| parse_secs(raw.format.as_ref().and_then(|f| f.duration.as_deref())))
        .unwrap_or(0.0);
    let frame_count = video
        .nb_frames
        .as_deref()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or_else(|| (duration * fps).round().max(0.0) as u64);

    Ok(MediaInfo {
        kind,
        width,
        height,
        rotation,
        duration,
        fps,
        frame_count,
        has_audio,
        codec,
        vfr,
    })
}

/// Exact number of decodable video frames (full decode, can be slow).
pub fn count_frames(path: &Path) -> Result<u64> {
    require_ffmpeg()?;
    let out = tool("ffprobe")
        .args([
            "-v", "error", "-count_frames", "-select_streams", "v:0", "-show_entries",
            "stream=nb_read_frames", "-of", "json",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()?;
    let bad = |m: String| MediaError::Probe {
        path: path.to_path_buf(),
        message: m,
    };
    if !out.status.success() {
        return Err(bad(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| bad(e.to_string()))?;
    v["streams"][0]["nb_read_frames"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| bad("no frame count in ffprobe output".into()))
}
