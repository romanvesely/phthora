//! Audio track decoding to PCM for analysis and preview playback.

use std::io::Read;
use std::path::Path;
use std::process::Stdio;

use crate::error::{MediaError, Result};
use crate::util::{ffmpeg_cmd, require_ffmpeg};

/// Channels of the first audio stream.
fn channels(path: &Path) -> Option<u32> {
    let out = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "a:0", "-show_entries", "stream=channels", "-of", "csv=p=0"])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// Decode the first audio stream to mono f32 samples at `sample_rate`, mixed down like
/// Web Audio's "speakers" down-mix (stereo: (L + R) / 2; 5.1: sqrt(1/2)(L + R) + C + (SL + SR) / 2).
pub fn decode_audio(path: &Path, sample_rate: u32) -> Result<Vec<f32>> {
    require_ffmpeg()?;
    // ffmpeg's own mono down-mix scales stereo by 1/sqrt(2)
    let mix = match channels(path) {
        Some(1) => "anull".to_string(),
        Some(2) => "pan=mono|c0=0.5*c0+0.5*c1".to_string(),
        Some(6) => "pan=mono|c0=0.7071*c0+0.7071*c1+c2+0.5*c4+0.5*c5".to_string(),
        _ => "aformat=channel_layouts=mono".to_string(),
    };
    let mut child = ffmpeg_cmd()
        .arg("-nostdin")
        .arg("-i")
        .arg(path)
        .args(["-map", "0:a:0", "-vn", "-af", &mix, "-ar", &sample_rate.to_string(), "-f", "f32le", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut bytes = Vec::new();
    child.stdout.take().unwrap().read_to_end(&mut bytes)?;
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err)?;
    let st = child.wait()?;
    if !st.success() {
        return Err(MediaError::Ffmpeg { context: format!("exit status {st}"), stderr: err.trim().to_string() });
    }
    Ok(bytes.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect())
}
