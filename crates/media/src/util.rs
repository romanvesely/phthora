//! Shared helpers: ffmpeg process setup, stderr collection, fps formatting.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{ChildStderr, Command};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

const STDERR_LINES: usize = 20;

/// True if both `ffmpeg` and `ffprobe` can be started.
pub fn ffmpeg_available() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| {
        ["ffmpeg", "ffprobe"].iter().all(|bin| {
            tool(bin)
                .arg("-version")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
    })
}

pub(crate) fn require_ffmpeg() -> crate::Result<()> {
    if ffmpeg_available() {
        Ok(())
    } else {
        Err(crate::MediaError::FfmpegMissing)
    }
}

/// Where ffmpeg or ffprobe is: in `$PHTHORA_FFMPEG_DIR`, next to the executable (or in
/// `ffmpeg/` beside it), in a macOS app bundle's `Resources/ffmpeg`, else found on PATH.
fn locate(bin: &str) -> PathBuf {
    let file = format!("{bin}{}", std::env::consts::EXE_SUFFIX);
    let mut dirs: Vec<PathBuf> = std::env::var_os("PHTHORA_FFMPEG_DIR").map(PathBuf::from).into_iter().collect();
    if let Ok(exe) = std::env::current_exe()
        && let Some(d) = exe.parent()
    {
        dirs.extend([d.to_path_buf(), d.join("ffmpeg"), d.join("../Resources/ffmpeg")]);
    }
    dirs.into_iter().map(|d| d.join(&file)).find(|p| p.is_file()).unwrap_or_else(|| PathBuf::from(bin))
}

/// A command for an external tool; on Windows it gets no console window of its own.
pub(crate) fn tool(bin: &str) -> Command {
    static FFMPEG: OnceLock<PathBuf> = OnceLock::new();
    static FFPROBE: OnceLock<PathBuf> = OnceLock::new();
    let path = match bin {
        "ffmpeg" => FFMPEG.get_or_init(|| locate(bin)).clone(),
        "ffprobe" => FFPROBE.get_or_init(|| locate(bin)).clone(),
        _ => PathBuf::from(bin),
    };
    #[allow(unused_mut)]
    let mut c = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

/// Base ffmpeg command: quiet, errors only.
pub(crate) fn ffmpeg_cmd() -> Command {
    let mut c = tool("ffmpeg");
    c.args(["-hide_banner", "-loglevel", "error"]);
    c
}

/// Format a frame rate for ffmpeg, mapping NTSC-style values to exact rationals
/// (29.97 -> 30000/1001) so frame timing stays exact.
pub(crate) fn fps_arg(fps: f64) -> String {
    let r = fps.round();
    if (fps - r).abs() < 1e-6 {
        return format!("{}", r as u64);
    }
    let n = (fps * 1.001).round();
    if n > 0.0 && (n * 1000.0 / 1001.0 - fps).abs() < 1e-4 {
        return format!("{}/1001", (n as u64) * 1000);
    }
    format!("{fps:.6}")
}

pub(crate) fn secs_arg(t: f64) -> String {
    format!("{:.6}", t.max(0.0))
}

/// Collects the last lines of a child's stderr on a background thread.
pub(crate) struct StderrTail {
    lines: Arc<Mutex<VecDeque<String>>>,
    handle: Option<JoinHandle<()>>,
}

impl StderrTail {
    pub(crate) fn spawn(stderr: ChildStderr) -> Self {
        let lines = Arc::new(Mutex::new(VecDeque::new()));
        let l2 = lines.clone();
        let handle = std::thread::spawn(move || {
            for line in BufReader::new(stderr).split(b'\n').map_while(|l| l.ok()) {
                let line = String::from_utf8_lossy(&line).trim_end().to_string();
                if line.is_empty() {
                    continue;
                }
                let mut q = l2.lock().unwrap();
                if q.len() >= STDERR_LINES {
                    q.pop_front();
                }
                q.push_back(line);
            }
        });
        Self {
            lines,
            handle: Some(handle),
        }
    }

    /// Current tail without waiting for the thread.
    pub(crate) fn snapshot(&self) -> String {
        let q = self.lines.lock().unwrap();
        q.iter().cloned().collect::<Vec<_>>().join("\n")
    }

    /// Wait for stderr to close, then return the tail.
    pub(crate) fn finish(mut self) -> String {
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        self.snapshot()
    }
}
