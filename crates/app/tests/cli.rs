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
fn list_shows_all_effects() {
    let out = run(bin().args(["render", "--list"]));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.lines().any(|l| l.starts_with("pixelate ")));
    assert!(s.lines().any(|l| l.starts_with("data_mosh ")));
    assert!(s.contains("audio_visualizer") && s.contains("push_draw") && s.contains("mask_draw"));
}

#[test]
fn unknown_effect_is_an_error() {
    let out = bin().args(["render", "--effect", "no_such_effect", "--out", "x.png"]).output().unwrap();
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

#[test]
fn help_lists_the_options() {
    for args in [&["--help"][..], &["render", "--help"], &["render", "-h"]] {
        let out = run(bin().args(args));
        assert!(out.status.success(), "{args:?}");
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("--audio FILE|none"), "{args:?}");
        for word in [".webp", ".apng", ".jpg", "%05d", "PNG sequence"] {
            assert!(text.contains(word), "{args:?} does not mention {word}");
        }
    }
}

/// Render a moving 64x36 clip with transparent blocks to `name` and return the ffprobe
/// fields of its first stream. The input moves: animated WebP merges identical frames.
fn render_clip(d: &Path, name: &str, extra: &[&str]) -> (PathBuf, String) {
    let out = d.join(name);
    let input = make_input(d, false);
    run(bin().arg("render").arg(&input).args(["--effect", "mask_blocks", "--size", "64x36", "--duration", "0.5", "--fps", "10", "--background", "transparent"]).args(extra).arg("--out").arg(&out));
    let info = probe(&out, "stream=codec_name,pix_fmt,width,height,nb_read_frames");
    (out, info)
}

fn alpha_pix_fmt(info: &str) -> bool {
    info.lines().find_map(|l| l.strip_prefix("pix_fmt=")).is_some_and(|p| p.contains('a'))
}

#[test]
fn animated_formats_by_extension_keep_alpha() {
    if !have("ffmpeg") || !have("ffprobe") {
        eprintln!("skipped: ffmpeg/ffprobe not found");
        return;
    }
    let d = tmp_dir("formats");
    for (name, codec) in [("a.webp", "webp_anim"), ("a.apng", "apng"), ("a.webm", "vp9")] {
        let (_, info) = render_clip(&d, name, &[]);
        // a width of 64 and 5 frames: 0.5 s at 10 fps
        assert!(info.contains(&format!("codec_name={codec}")) && info.contains("width=64") && info.contains("height=36") && info.contains("nb_read_frames=5"), "{name}: {info}");
        if name != "a.webm" {
            assert!(alpha_pix_fmt(&info), "{name} lost its alpha: {info}");
        }
    }
    // GIF keeps transparency too, and takes at most 50 fps
    let out = d.join("fast.gif");
    let res = run(bin().arg("render").arg(d.join("in.mp4")).args(["--effect", "mask_blocks", "--size", "64x36", "--duration", "0.5", "--fps", "60", "--background", "transparent", "--out"]).arg(&out));
    assert!(String::from_utf8_lossy(&res.stderr).contains("at most 50 fps"));
    let info = probe(&out, "stream=codec_name,nb_read_frames,r_frame_rate");
    assert!(info.contains("codec_name=gif") && info.contains("nb_read_frames=25") && info.contains("r_frame_rate=50/1"), "{info}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn png_sequence_by_folder_and_pattern() {
    if !have("ffmpeg") || !have("ffprobe") {
        eprintln!("skipped: ffmpeg/ffprobe not found");
        return;
    }
    let d = tmp_dir("seq");
    let common = ["render", "--effect", "mask_blocks", "--size", "64x36", "--duration", "0.5", "--fps", "10", "--background", "transparent", "--out"];
    // a trailing slash names a folder
    let folder = format!("{}/shots/", d.display());
    run(bin().args(common).arg(&folder));
    let names = |dir: &Path| {
        let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    };
    assert_eq!(names(&d.join("shots")), (1..=5).map(|n| format!("shots-{n:05}.png")).collect::<Vec<_>>());
    let info = probe(&d.join("shots/shots-00001.png"), "stream=codec_name,pix_fmt,width,height");
    assert!(info.contains("codec_name=png") && info.contains("width=64") && info.contains("height=36") && alpha_pix_fmt(&info), "{info}");
    // a %05d name is a pattern
    run(bin().args(common).arg(d.join("p/f_%05d.png")));
    assert_eq!(names(&d.join("p")).len(), 5);
    assert!(d.join("p/f_00005.png").exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn stills_by_extension() {
    if !have("ffmpeg") || !have("ffprobe") {
        eprintln!("skipped: ffmpeg/ffprobe not found");
        return;
    }
    let d = tmp_dir("stills");
    // without --duration or --fps a .webp is a still: one frame, with alpha
    let still = d.join("still.webp");
    run(bin().args(["render", "--effect", "mask_blocks", "--size", "64x36", "--background", "transparent", "--out"]).arg(&still));
    let info = probe(&still, "stream=codec_name,pix_fmt,width,height,nb_read_frames");
    assert!(info.contains("codec_name=webp\n") && info.contains("nb_read_frames=1") && alpha_pix_fmt(&info), "{info}");
    let jpg = d.join("still.jpg");
    run(bin().args(["render", "--effect", "color_gradient", "--size", "64x36", "--quality", "low", "--out"]).arg(&jpg));
    let info = probe(&jpg, "stream=codec_name,width,height,nb_read_frames");
    assert!(info.contains("codec_name=mjpeg") && info.contains("width=64") && info.contains("height=36") && info.contains("nb_read_frames=1"), "{info}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn unknown_output_names_are_rejected() {
    for name in ["x.txt", "x", "frame-%05d.jpg"] {
        let out = bin().args(["render", "--effect", "pixelate", "--out", name]).output().unwrap();
        assert!(!out.status.success(), "{name} was accepted");
        assert!(String::from_utf8_lossy(&out.stderr).contains("output must end in"), "{name}");
    }
}

#[test]
fn version() {
    for flag in ["--version", "-V"] {
        let out = run(bin().arg(flag));
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), format!("phthora {}", env!("CARGO_PKG_VERSION")));
    }
}

#[test]
fn caption_uses_fonts_from_the_fonts_folder() {
    if !have("ffmpeg") {
        eprintln!("skipped: ffmpeg not found");
        return;
    }
    let d = tmp_dir("fonts");
    let data = d.join("data");
    std::fs::create_dir_all(data.join("phthora/fonts")).unwrap();
    let mono = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/roboto-mono-regular.ttf");
    std::fs::copy(&mono, data.join("phthora/fonts/Custom.ttf")).unwrap();
    let render = |font: &str| {
        let preset = d.join(format!("{font}.phthora.json"));
        let json = format!(r#"{{"version":2,"effects":[{{"effect":"caption","enabled":true,"locked":false,"expanded":true,"seed":0.5,"params":{{"text":"Hello 123","font":"{font}","size":0.5}}}}]}}"#);
        std::fs::write(&preset, json).unwrap();
        let out = d.join(format!("{font}.png"));
        let res = run(bin().env("XDG_DATA_HOME", &data).args(["render", "--size", "160x90", "--preset"]).arg(&preset).arg("--out").arg(&out));
        (std::fs::read(&out).unwrap(), String::from_utf8_lossy(&res.stderr).to_string())
    };
    let (custom, log) = render("Custom");
    assert!(!log.contains("ignored param"), "the font name is accepted: {log}");
    let (mono, _) = render("Mono");
    let (sans, _) = render("Sans");
    assert_eq!(custom, mono, "Custom.ttf is the mono font");
    assert_ne!(custom, sans);
    // a font that is gone falls back to the default
    let (gone, _) = render("Gone");
    assert_eq!(gone, sans);
}
