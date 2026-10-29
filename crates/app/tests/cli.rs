//! Command line rendering, end to end: effect list, still export and video export.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_phthora"))
}

fn tmp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("phthora-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg("-version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn run(cmd: &mut Command) -> Output {
    let out = cmd.output().expect("run phthora");
    assert!(out.status.success(), "phthora failed:\n{}", String::from_utf8_lossy(&out.stderr));
    out
}

fn probe(path: &Path, entries: &str) -> String {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-count_frames", "-select_streams", "v:0", "-show_entries", entries, "-of", "default=nw=1"])
        .arg(path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn list_hides_unavailable_effects() {
    let out = run(bin().args(["render", "--list"]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.lines().any(|l| l.starts_with("pixelate ")));
    assert!(s.lines().any(|l| l.starts_with("data_mosh ")));
    assert!(!s.contains("push_draw") && !s.contains("mask_draw"));
    assert!(s.contains("audio_visualizer"));
}

#[test]
fn unknown_effect_is_an_error() {
    let out = bin().args(["render", "--effect", "push_draw", "--out", "x.png"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown effect"));
}

#[test]
fn still_png_without_input() {
    if !have("ffmpeg") {
        eprintln!("skipped: ffmpeg not found");
        return;
    }
    let d = tmp_dir("png");
    let out = d.join("still.png");
    run(bin().args(["render", "--effect", "color_gradient,pixelate", "--size", "64x36", "--out"]).arg(&out));
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    // IHDR width and height
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 64);
    assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 36);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn video_export_from_preset_is_deterministic() {
    if !have("ffmpeg") || !have("ffprobe") {
        eprintln!("skipped: ffmpeg/ffprobe not found");
        return;
    }
    let d = tmp_dir("mp4");
    let input = d.join("in.mp4");
    let st = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=96x54:rate=10:duration=1", "-pix_fmt", "yuv420p"])
        .arg(&input)
        .status()
        .unwrap();
    assert!(st.success());
    let preset = d.join("chain.phthora.json");
    std::fs::write(
        &preset,
        r#"{"version":1,"effects":[
            {"effect":"rgb_shift","seed":0.25,"params":{"amount":0.3},
             "modulators":{"amount":{"enabled":true,"shape":"sine","speed":1.0,"min":0.0,"max":1.0,"loop":true}}},
            {"effect":"feedback","seed":0.5}
        ]}"#,
    )
    .unwrap();
    let render = |name: &str| {
        let out = d.join(name);
        run(bin().arg("render").arg(&input).arg("--preset").arg(&preset).args(["--fps", "10", "--quality", "low", "--out"]).arg(&out));
        out
    };
    let a = render("a.mp4");
    let b = render("b.mp4");
    let info = probe(&a, "stream=width,height,nb_read_frames,r_frame_rate");
    assert!(info.contains("width=96") && info.contains("height=54"), "{info}");
    assert!(info.contains("nb_read_frames=10"), "{info}");
    assert!(info.contains("r_frame_rate=10/1"), "{info}");
    // same chain, same seeds, same input: identical decoded frames
    let frames = |p: &Path| {
        Command::new("ffmpeg").args(["-v", "error", "-i"]).arg(p).args(["-f", "rawvideo", "-pix_fmt", "rgb24", "-"]).output().unwrap().stdout
    };
    let (fa, fb) = (frames(&a), frames(&b));
    assert_eq!(fa.len(), 96 * 54 * 3 * 10);
    assert!(fa == fb, "two renders of the same preset differ");
    let _ = std::fs::remove_dir_all(&d);
}

fn make_input(d: &Path, audio: bool) -> PathBuf {
    let input = d.join("in.mp4");
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=96x54:rate=10:duration=2"]);
    if audio {
        cmd.args(["-f", "lavfi", "-i", "sine=d=2", "-c:a", "aac", "-shortest"]);
    }
    assert!(cmd.args(["-pix_fmt", "yuv420p"]).arg(&input).status().unwrap().success());
    input
}

fn streams(path: &Path) -> String {
    let out = Command::new("ffprobe").args(["-v", "error", "-show_entries", "stream=codec_type,pix_fmt", "-of", "default=nw=1"]).arg(path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn trim_speed_and_direction_set_the_length() {
    if !have("ffmpeg") || !have("ffprobe") {
        eprintln!("skipped: ffmpeg/ffprobe not found");
        return;
    }
    let d = tmp_dir("trim");
    let input = make_input(&d, true);
    let frames = |extra: &[&str], name: &str| {
        let out = d.join(name);
        run(bin().arg("render").arg(&input).args(extra).args(["--quality", "low", "--backend", "software", "--out"]).arg(&out));
        (probe(&out, "stream=nb_read_frames"), streams(&out))
    };
    // 0.5 s of input at 10 fps
    let (f, s) = frames(&["--trim", "0.5", "1.0"], "trim.mp4");
    assert!(f.contains("nb_read_frames=5"), "{f}");
    assert!(s.contains("codec_type=audio"), "trimmed input keeps its audio: {s}");
    // ping-pong: 2 * 20 - 2 frames
    let (f, s) = frames(&["--pingpong"], "pp.mp4");
    assert!(f.contains("nb_read_frames=38"), "{f}");
    assert!(!s.contains("codec_type=audio"), "re-timed input drops its audio: {s}");
    // double speed: half the length
    let (f, _) = frames(&["--speed", "2"], "fast.mp4");
    assert!(f.contains("nb_read_frames=10"), "{f}");
    let (_, s) = frames(&["--audio", "none"], "silent.mp4");
    assert!(!s.contains("codec_type=audio"), "{s}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn transparent_background_keeps_alpha() {
    if !have("ffmpeg") || !have("ffprobe") {
        eprintln!("skipped: ffmpeg/ffprobe not found");
        return;
    }
    let d = tmp_dir("alpha");
    let out = d.join("a.webm");
    run(bin().args(["render", "--effect", "mask_blocks", "--size", "64x36", "--duration", "0.5", "--background", "transparent", "--out"]).arg(&out));
    // VP9 alpha is stored as side data; the stream tag says so
    let tags = Command::new("ffprobe").args(["-v", "error", "-show_entries", "stream_tags=alpha_mode", "-of", "default=nw=1"]).arg(&out).output().unwrap();
    assert!(String::from_utf8_lossy(&tags.stdout).contains("alpha_mode=1"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn bad_options_are_rejected() {
    for args in [
        &["--speed", "9"][..],
        &["--fps", "0"],
        &["--size", "8x8"],
        &["--background", "#12"],
        &["--backend", "magic"],
        &["--duration", "-1"],
    ] {
        let out = bin().arg("render").args(args).args(["--out", "x.mp4"]).output().unwrap();
        assert!(!out.status.success(), "{args:?} was accepted");
    }
}
