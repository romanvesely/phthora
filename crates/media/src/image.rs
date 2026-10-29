use std::path::Path;
use std::process::Stdio;

use crate::error::{MediaError, Result};
use crate::probe::probe;
use crate::size::fit_size;
use crate::util::{StderrTail, ffmpeg_cmd, require_ffmpeg};

/// Decoded still image: top-down RGBA8, straight (non-premultiplied) alpha.
#[derive(Debug, Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Decode the first frame of `path` to RGBA, scaled via [`fit_size`] so the
/// longer side is at most `max_dim` (sizes are made even). EXIF/display
/// rotation is applied by ffmpeg.
pub fn load_image(path: &Path, max_dim: u32) -> Result<RgbaImage> {
    let info = probe(path)?;
    let (w, h) = fit_size(info.width, info.height, max_dim);
    decode_still(path, info.width, info.height, w, h)
}

/// Decode one frame at `(w, h)`; `(src_w, src_h)` is the display size, used to
/// skip the scaler when sizes already match.
pub(crate) fn decode_still(path: &Path, src_w: u32, src_h: u32, w: u32, h: u32) -> Result<RgbaImage> {
    require_ffmpeg()?;
    let mut cmd = ffmpeg_cmd();
    cmd.arg("-nostdin").arg("-i").arg(path).args(["-map", "0:v:0", "-an", "-sn", "-frames:v", "1"]);
    if (w, h) != (src_w, src_h) {
        cmd.args(["-vf", &format!("scale={w}:{h}")]);
    }
    cmd.args(["-pix_fmt", "rgba", "-f", "rawvideo", "-"]);
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let tail = StderrTail::spawn(child.stderr.take().unwrap());
    let mut data = Vec::with_capacity(w as usize * h as usize * 4);
    {
        use std::io::Read;
        child.stdout.take().unwrap().read_to_end(&mut data)?;
    }
    let status = child.wait()?;
    let stderr = tail.finish();
    let expected = w as usize * h as usize * 4;
    if !status.success() || data.len() != expected {
        return Err(MediaError::Ffmpeg {
            context: format!("decoding {}", path.display()),
            stderr: if stderr.is_empty() {
                format!("got {} bytes, expected {expected}", data.len())
            } else {
                stderr
            },
        });
    }
    Ok(RgbaImage {
        width: w,
        height: h,
        data,
    })
}
