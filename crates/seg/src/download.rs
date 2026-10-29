//! Model download through `curl`, verified by size and SHA-256, stored atomically.

use crate::{set_download, set_load, Error, Load, MODEL};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Download the model into `models_dir()` unless it is already there and intact.
/// `progress(done, total)` is called about ten times a second; setting `cancel` aborts.
/// The file only appears at `model_path()` once the checksum matches.
pub fn download(mut progress: impl FnMut(u64, u64), cancel: &AtomicBool) -> Result<PathBuf, Error> {
    let dest = crate::model_path();
    if crate::is_installed() && verify_file(&dest, MODEL.sha256).is_ok() {
        progress(MODEL.size, MODEL.size);
        return Ok(dest);
    }
    set_download(Some((0, MODEL.size)));
    let r = fetch(MODEL.url, &dest, MODEL.sha256, MODEL.size, &mut progress, cancel);
    set_download(None);
    r?;
    // a new file gets a fresh load attempt, even after an earlier failure
    set_load(Load::Idle);
    Ok(dest)
}

/// Check a file against a lowercase hex SHA-256.
pub fn verify_file(path: &Path, sha256: &str) -> Result<(), Error> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let actual: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if actual == sha256 { Ok(()) } else { Err(Error::Checksum { expected: sha256.into(), actual }) }
}

pub(crate) fn fetch(
    url: &str,
    dest: &Path,
    sha256: &str,
    size: u64,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let dir = dest.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = dest.file_name().and_then(|n| n.to_str()).unwrap_or("model");
    let tmp = dir.join(format!("{name}.part-{}", std::process::id()));
    let r = fetch_to(url, &tmp, sha256, size, progress, cancel).and_then(|_| std::fs::rename(&tmp, dest).map_err(Error::from));
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

fn fetch_to(url: &str, tmp: &Path, sha256: &str, size: u64, progress: &mut dyn FnMut(u64, u64), cancel: &AtomicBool) -> Result<(), Error> {
    // https only; file:// is for tests
    let proto = if url.starts_with("file://") { "=file" } else { "=https" };
    let mut cmd = Command::new("curl");
    #[cfg(windows)]
    {
        // no console window of its own
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.args(["--fail", "--location", "--silent", "--show-error", "--proto", proto, "--proto-redir", proto])
        .args(["--connect-timeout", "20", "--max-time", "900"])
        .arg("--output")
        .arg(tmp);
    if size > 0 {
        cmd.arg("--max-filesize").arg((size * 2).to_string());
    }
    cmd.arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => Error::Download("curl is not installed".into()),
        _ => Error::Io(e),
    })?;
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Cancelled);
        }
        if let Some(s) = child.try_wait()? {
            break s;
        }
        let done = std::fs::metadata(tmp).map(|m| m.len()).unwrap_or(0);
        set_download(Some((done, size)));
        progress(done, size);
        std::thread::sleep(Duration::from_millis(100));
    };
    if !status.success() {
        let mut msg = String::new();
        if let Some(mut e) = child.stderr.take() {
            let _ = e.read_to_string(&mut msg);
        }
        let msg = msg.trim();
        return Err(Error::Download(if msg.is_empty() { format!("curl {status}") } else { msg.to_string() }));
    }
    let len = std::fs::metadata(tmp)?.len();
    progress(len, size.max(len));
    if size > 0 && len != size {
        return Err(Error::Download(format!("unexpected size {len} (expected {size})")));
    }
    verify_file(tmp, sha256)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("phthora-seg-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sha(data: &[u8]) -> String {
        Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
    }

    fn curl_available() -> bool {
        Command::new("curl").arg("--version").stdout(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
    }

    #[test]
    fn checksum_decides_whether_the_file_is_kept() {
        if !curl_available() {
            eprintln!("skipped: curl not found");
            return;
        }
        let dir = scratch("dl");
        let data: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();
        let src = dir.join("src.bin");
        std::fs::write(&src, &data).unwrap();
        let url = format!("file://{}", src.display());
        let dest = dir.join("out").join("model.bin");
        let cancel = AtomicBool::new(false);

        // wrong checksum: error, nothing at the destination, no leftovers
        let wrong = sha(b"something else");
        let err = fetch(&url, &dest, &wrong, data.len() as u64, &mut |_, _| {}, &cancel).unwrap_err();
        assert!(matches!(err, Error::Checksum { .. }), "{err}");
        assert!(!dest.exists());
        assert_eq!(std::fs::read_dir(dest.parent().unwrap()).unwrap().count(), 0, "temp file removed");

        // wrong size is caught before hashing
        let err = fetch(&url, &dest, &sha(&data), 123, &mut |_, _| {}, &cancel).unwrap_err();
        assert!(matches!(err, Error::Download(_)), "{err}");
        assert!(!dest.exists());

        // right checksum: file in place, progress reported
        let mut last = (0, 0);
        fetch(&url, &dest, &sha(&data), data.len() as u64, &mut |d, t| last = (d, t), &cancel).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), data);
        assert_eq!(last, (data.len() as u64, data.len() as u64));
        assert!(verify_file(&dest, &sha(&data)).is_ok());
        assert!(verify_file(&dest, &wrong).is_err());

        // a missing source is an error, too
        let err = fetch("file:///nonexistent/phthora", &dir.join("x.bin"), &wrong, 0, &mut |_, _| {}, &cancel).unwrap_err();
        assert!(matches!(err, Error::Download(_)), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancel_stops_the_download() {
        if !curl_available() {
            return;
        }
        let dir = scratch("cancel");
        let cancel = AtomicBool::new(true);
        let err = fetch("file:///dev/zero", &dir.join("m.bin"), &sha(b""), 0, &mut |_, _| {}, &cancel).unwrap_err();
        assert!(matches!(err, Error::Cancelled));
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn model_spec_is_consistent() {
        assert_eq!(MODEL.sha256.len(), 64);
        assert!(MODEL.sha256.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        assert!(MODEL.url.starts_with("https://"));
    }
}
