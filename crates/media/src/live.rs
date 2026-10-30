//! Live video through V4L2: cameras as input (optionally recorded to a file by the same
//! ffmpeg process) and a virtual camera (v4l2loopback) as output for other programs.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::decoder::Frame;
use crate::util::{StderrTail, ffmpeg_cmd, fps_arg, require_ffmpeg};
use crate::{MediaError, Result};

/// Preferred camera mode.
const CAMERA_SIZE: (u32, u32) = (1280, 720);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoDevice {
    pub path: PathBuf,
    pub name: String,
    /// delivers video (a camera, or a loopback device someone writes to)
    pub capture: bool,
    /// a v4l2loopback device: can serve as virtual camera
    pub loopback: bool,
}

/// Video devices, by device number. Opening a device for this does not start the camera.
pub fn list_devices() -> Vec<VideoDevice> {
    let mut v: Vec<(u32, VideoDevice)> = std::fs::read_dir("/dev")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let n: u32 = name.strip_prefix("video")?.parse().ok()?;
            let (driver, card, caps) = querycap(&e.path())?;
            let loopback = driver.contains("loopback");
            // V4L2_CAP_VIDEO_CAPTURE; metadata-only nodes of a camera are skipped
            let capture = caps & 0x1 != 0;
            (capture || loopback).then(|| (n, VideoDevice { path: e.path(), name: card, capture, loopback }))
        })
        .collect();
    v.sort_by_key(|(n, _)| *n);
    v.into_iter().map(|(_, d)| d).collect()
}

/// (driver, card name, device capabilities) of a V4L2 node.
#[cfg(target_os = "linux")]
fn querycap(path: &Path) -> Option<(String, String, u32)> {
    use std::os::fd::AsRawFd;
    // struct v4l2_capability: driver[16] card[32] bus_info[32] version capabilities device_caps reserved[3]
    const VIDIOC_QUERYCAP: libc::c_ulong = 0x8068_5600;
    let f = std::fs::OpenOptions::new().read(true).write(true).custom_flags(libc::O_NONBLOCK).open(path).or_else(|_| std::fs::File::open(path)).ok()?;
    let mut cap = [0u8; 104];
    // SAFETY: the buffer has the size the ioctl writes, the fd is open for the call
    let r = unsafe { libc::ioctl(f.as_raw_fd(), VIDIOC_QUERYCAP as _, cap.as_mut_ptr()) };
    if r != 0 {
        return None;
    }
    let s = |b: &[u8]| String::from_utf8_lossy(b.split(|c| *c == 0).next().unwrap_or(b)).trim().to_string();
    let u = |o: usize| u32::from_ne_bytes(cap[o..o + 4].try_into().unwrap());
    let (caps, device_caps) = (u(84), u(88));
    // device_caps is valid when V4L2_CAP_DEVICE_CAPS is set
    let c = if caps & 0x8000_0000 != 0 { device_caps } else { caps };
    Some((s(&cap[0..16]), s(&cap[16..48]), c))
}

#[cfg(not(target_os = "linux"))]
fn querycap(_path: &Path) -> Option<(String, String, u32)> {
    None
}

#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;

/// Input options for a camera: 1280x720 MJPEG if offered (raw YUYV is slow at that size
/// over USB), else the camera's default mode.
fn capture_args(dev: &Path) -> Vec<String> {
    let out = crate::util::tool("ffmpeg")
        .args(["-hide_banner", "-f", "v4l2", "-list_formats", "all", "-i"])
        .arg(dev)
        .stdin(Stdio::null())
        .output();
    let text = out.map(|o| String::from_utf8_lossy(&o.stderr).to_string()).unwrap_or_default();
    let size = format!("{}x{}", CAMERA_SIZE.0, CAMERA_SIZE.1);
    for fmt in ["mjpeg", "yuyv422"] {
        let offers = text.lines().any(|l| {
            let mut parts = l.split(':').map(str::trim);
            parts.nth(1) == Some(fmt) && l.split_whitespace().any(|w| w == size)
        });
        if offers {
            return vec!["-input_format".into(), fmt.into(), "-video_size".into(), size];
        }
    }
    Vec::new()
}

/// Record the camera (and the default microphone) while showing it.
#[derive(Clone, Debug)]
pub struct Record {
    pub path: PathBuf,
    pub audio: bool,
}

#[derive(Default)]
struct LiveState {
    latest: Option<Arc<Frame>>,
    seq: u64,
    error: Option<String>,
    ended: bool,
}

/// A running camera: the newest frame is always available without waiting.
pub struct LiveSource {
    state: Arc<Mutex<LiveState>>,
    child: Arc<Mutex<Option<Child>>>,
    stdin: Option<ChildStdin>,
    thread: Option<JoinHandle<()>>,
    size: (u32, u32),
    record: Option<Record>,
}

impl LiveSource {
    /// Show camera `dev` scaled to `w` x `h`, optionally recording it at its own size.
    pub fn open(dev: &Path, w: u32, h: u32, record: Option<Record>) -> Result<Self> {
        require_ffmpeg()?;
        let mut cmd = ffmpeg_cmd();
        cmd.args(["-thread_queue_size", "64", "-f", "v4l2"]).args(capture_args(dev)).arg("-i").arg(dev);
        let audio = record.as_ref().is_some_and(|r| r.audio);
        if audio {
            cmd.args(["-thread_queue_size", "1024", "-f", "pulse", "-i", "default"]);
        }
        cmd.args(["-map", "0:v", "-vf", &format!("scale={w}:{h}"), "-pix_fmt", "rgba", "-f", "rawvideo", "pipe:1"]);
        if let Some(r) = &record {
            cmd.args(["-map", "0:v"]);
            if audio {
                cmd.args(["-map", "1:a", "-c:a", "aac", "-b:a", "192k"]);
            }
            // written out packet by packet: a killed ffmpeg still leaves a readable file
            cmd.args(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "16", "-pix_fmt", "yuv420p", "-flush_packets", "1", "-y"]).arg(&r.path);
        }
        let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(MediaError::Io)?;
        let mut stdout = child.stdout.take().unwrap();
        let stdin = child.stdin.take();
        let tail = StderrTail::spawn(child.stderr.take().unwrap());
        let state = Arc::new(Mutex::new(LiveState::default()));
        let child = Arc::new(Mutex::new(Some(child)));
        let st = state.clone();
        let thread = std::thread::Builder::new()
            .name("phthora-live".into())
            .spawn(move || {
                let len = w as usize * h as usize * 4;
                loop {
                    let mut buf = vec![0u8; len];
                    if stdout.read_exact(&mut buf).is_err() {
                        break;
                    }
                    let mut s = st.lock().unwrap();
                    let f = Frame { index: s.seq, pass: 0, pts: 0.0, width: w, height: h, data: buf };
                    s.latest = Some(Arc::new(f));
                    s.seq += 1;
                }
                let err = tail.finish();
                let mut s = st.lock().unwrap();
                s.ended = true;
                if !err.is_empty() {
                    s.error.get_or_insert(err);
                }
            })
            .map_err(MediaError::Io)?;
        Ok(Self { state, child, stdin, thread: Some(thread), size: (w, h), record })
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// The newest frame and its number (counts up with every camera frame).
    pub fn latest(&self) -> Option<(u64, Arc<Frame>)> {
        let s = self.state.lock().unwrap();
        s.latest.clone().map(|f| (s.seq - 1, f))
    }

    /// Why the camera stopped, if it did.
    pub fn error(&self) -> Option<String> {
        let s = self.state.lock().unwrap();
        match (&s.error, s.ended) {
            (Some(e), _) => Some(e.clone()),
            (None, true) => Some("camera stopped".into()),
            _ => None,
        }
    }

    pub fn recording(&self) -> Option<&Path> {
        self.record.as_ref().map(|r| r.path.as_path())
    }

    /// Stop the camera; a recording is finished properly. Returns the recorded file.
    pub fn stop(mut self) -> Option<PathBuf> {
        self.shutdown();
        self.record.take().map(|r| r.path).filter(|p| p.exists())
    }

    fn shutdown(&mut self) {
        // "q" lets ffmpeg finish the recording; kill only if it does not react
        if let Some(mut i) = self.stdin.take() {
            let _ = i.write_all(b"q");
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        if let Some(mut c) = self.child.lock().unwrap().take() {
            loop {
                match c.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if std::time::Instant::now() < deadline => std::thread::sleep(std::time::Duration::from_millis(20)),
                    _ => {
                        let _ = c.kill();
                        let _ = c.wait();
                        break;
                    }
                }
            }
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for LiveSource {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Frames sent to a v4l2loopback device, where other programs see them as a camera.
pub struct VirtualCam {
    tx: Option<SyncSender<Vec<u8>>>,
    thread: Option<JoinHandle<Option<String>>>,
    child: Arc<Mutex<Option<Child>>>,
    size: (u32, u32),
    error: Arc<Mutex<Option<String>>>,
}

impl VirtualCam {
    pub fn open(dev: &Path, w: u32, h: u32, fps: f64) -> Result<Self> {
        require_ffmpeg()?;
        let mut child = ffmpeg_cmd()
            .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{w}x{h}"), "-r", &fps_arg(fps), "-i", "pipe:0"])
            .args(["-f", "v4l2", "-pix_fmt", "yuv420p"])
            .arg(dev)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(MediaError::Io)?;
        let mut stdin = child.stdin.take().unwrap();
        let tail = StderrTail::spawn(child.stderr.take().unwrap());
        // one frame of slack: a slow consumer drops frames instead of stalling the preview
        let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(1);
        let error = Arc::new(Mutex::new(None));
        let err2 = error.clone();
        let thread = std::thread::Builder::new()
            .name("phthora-vcam".into())
            .spawn(move || {
                for f in rx {
                    if stdin.write_all(&f).is_err() {
                        break;
                    }
                }
                drop(stdin);
                let e = tail.finish();
                let e = (!e.is_empty()).then_some(e);
                if let Some(e) = &e {
                    err2.lock().unwrap().get_or_insert(e.clone());
                }
                e
            })
            .map_err(MediaError::Io)?;
        Ok(Self { tx: Some(tx), thread: Some(thread), child: Arc::new(Mutex::new(Some(child))), size: (w, h), error })
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Queue a top-down RGBA frame. Returns false if it was dropped (consumer busy or gone).
    pub fn push(&self, rgba: Vec<u8>) -> bool {
        match self.tx.as_ref().map(|t| t.try_send(rgba)) {
            Some(Ok(())) => true,
            Some(Err(TrySendError::Full(_))) => false,
            _ => false,
        }
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }
}

impl Drop for VirtualCam {
    fn drop(&mut self) {
        drop(self.tx.take());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        if let Some(mut c) = self.child.lock().unwrap().take() {
            let _ = c.wait();
        }
    }
}
