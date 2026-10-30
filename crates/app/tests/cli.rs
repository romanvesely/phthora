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
    // each category in one block, like the editor's list
    let cats: Vec<&str> = s.lines().map(|l| l.split_whitespace().nth(1).unwrap_or("")).collect();
    let blocks = cats.windows(2).filter(|w| w[0] != w[1]).count() + 1;
    let distinct: std::collections::HashSet<_> = cats.iter().collect();
    assert_eq!(blocks, distinct.len());
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
        for word in [".webp", ".apng", ".jpg", "%05d", "PNG sequence", "--out-dir DIR", "--format EXT"] {
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

/// Size and frame count from the chunks of an animated WebP (frames may cover only the
/// changed area, so the transparency is not visible from the header).
fn webp_anim(path: &Path) -> Option<(u32, u32, usize)> {
    let b = std::fs::read(path).ok()?;
    if b.get(0..4)? != b"RIFF" || b.get(8..16)? != b"WEBPVP8X" {
        return None;
    }
    let u24 = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], 0]) + 1;
    let (w, h) = (u24(24), u24(27));
    let (mut at, mut frames, mut anim) = (12, 0, false);
    while at + 8 <= b.len() {
        let len = u32::from_le_bytes(b[at + 4..at + 8].try_into().ok()?) as usize;
        match &b[at..at + 4] {
            b"ANIM" => anim = true,
            b"ANMF" => frames += 1,
            _ => {}
        }
        at += 8 + len + len % 2;
    }
    anim.then_some((w, h, frames))
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
        let (out, info) = render_clip(&d, name, &[]);
        if name == "a.webp" && info.contains("width=0") {
            // ffprobe before 7.1 cannot decode animated WebP: read the container instead
            assert_eq!(webp_anim(&out), Some((64, 36, 5)), "{name}");
            continue;
        }
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
        // "phthora 1.1.0 (2026-10-30 abc1234)"
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let rest = s.strip_prefix(&format!("phthora {} (", env!("CARGO_PKG_VERSION"))).unwrap_or_else(|| panic!("{s}"));
        let (date, commit) = rest.trim_end_matches(')').split_once(' ').unwrap();
        assert!(date.len() == 10 && date.as_bytes()[4] == b'-', "{s}");
        assert!(commit == "unknown" || commit.len() == 7, "{s}");
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

/// A small test clip or image made by ffmpeg.
fn make_media(path: &Path, source: &str, extra: &[&str]) {
    let st = Command::new("ffmpeg").args(["-v", "error", "-y", "-f", "lavfi", "-i", source]).args(extra).arg(path).status().unwrap();
    assert!(st.success(), "ffmpeg could not make {}", path.display());
}

fn listing(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    v.sort();
    v
}

#[test]
fn batch_renders_every_input_with_its_own_size_and_format() {
    if !have("ffmpeg") || !have("ffprobe") {
        eprintln!("skipped: ffmpeg/ffprobe not found");
        return;
    }
    let d = tmp_dir("batch");
    let (a, b, c) = (d.join("a.mp4"), d.join("b.webm"), d.join("c.png"));
    make_media(&a, "testsrc2=size=96x54:rate=10:duration=1", &["-pix_fmt", "yuv420p"]);
    make_media(&b, "testsrc2=size=64x36:rate=5:duration=2", &["-c:v", "libvpx-vp9"]);
    make_media(&c, "testsrc2=size=80x48:rate=1", &["-frames:v", "1"]);
    let out_dir = d.join("out");
    let res = run(bin().arg("render").args([&a, &b, &c]).args(["--effect", "pixelate", "--quality", "low", "--backend", "software", "--out-dir"]).arg(&out_dir));
    let log = String::from_utf8_lossy(&res.stderr);
    assert!(log.contains("[1/3]") && log.contains("[3/3]") && log.contains("3 of 3 exported"), "{log}");
    let names = listing(&out_dir);
    assert_eq!(names.len(), 3, "{names:?}");
    let find = |stem: &str, ext: &str| out_dir.join(names.iter().find(|n| n.starts_with(&format!("{stem}-phthora-")) && n.ends_with(ext)).unwrap_or_else(|| panic!("no {stem}*{ext} in {names:?}")));
    // each output has the size, length and frame rate of its own input
    let info = probe(&find("a", ".mp4"), "stream=codec_name,width,height,nb_read_frames");
    assert!(info.contains("codec_name=h264") && info.contains("width=96") && info.contains("height=54") && info.contains("nb_read_frames=10"), "{info}");
    let info = probe(&find("b", ".webm"), "stream=codec_name,width,height,nb_read_frames,r_frame_rate");
    assert!(info.contains("codec_name=vp9") && info.contains("width=64") && info.contains("height=36") && info.contains("nb_read_frames=10") && info.contains("r_frame_rate=5/1"), "{info}");
    let info = probe(&find("c", ".png"), "stream=codec_name,width,height,nb_read_frames");
    assert!(info.contains("codec_name=png") && info.contains("width=80") && info.contains("height=48") && info.contains("nb_read_frames=1"), "{info}");

    // a second run into the same folder never overwrites: same stems, new names
    let before = names.clone();
    run(bin().arg("render").args([&a, &b, &c]).args(["--effect", "pixelate", "--quality", "low", "--backend", "software", "--out-dir"]).arg(&out_dir));
    let after = listing(&out_dir);
    assert_eq!(after.len(), 6, "{after:?}");
    assert!(before.iter().all(|n| after.contains(n)));

    // --format and the overrides hold for all inputs
    let gif_dir = d.join("gifs");
    run(bin().arg("render").args([&a, &c]).args(["--effect", "pixelate", "--format", "gif", "--size", "48x28", "--duration", "0.5", "--fps", "10", "--out-dir"]).arg(&gif_dir));
    let gifs = listing(&gif_dir);
    assert_eq!(gifs.len(), 2);
    for g in &gifs {
        let info = probe(&gif_dir.join(g), "stream=codec_name,width,height,nb_read_frames");
        assert!(g.ends_with(".gif") && info.contains("codec_name=gif") && info.contains("width=48") && info.contains("height=28") && info.contains("nb_read_frames=5"), "{g}: {info}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn batch_goes_on_after_a_broken_input_and_fails_at_the_end() {
    if !have("ffmpeg") || !have("ffprobe") {
        eprintln!("skipped: ffmpeg/ffprobe not found");
        return;
    }
    let d = tmp_dir("batch-bad");
    let (good1, bad, missing, good2) = (d.join("one.mp4"), d.join("broken.mp4"), d.join("missing.mp4"), d.join("two.mp4"));
    make_media(&good1, "testsrc2=size=64x36:rate=10:duration=1", &["-pix_fmt", "yuv420p"]);
    make_media(&good2, "testsrc2=size=64x36:rate=10:duration=1", &["-pix_fmt", "yuv420p"]);
    std::fs::write(&bad, b"this is not a video").unwrap();
    let out_dir = d.join("out");
    let out = bin()
        .arg("render")
        .args([&good1, &bad, &missing, &good2])
        .args(["--effect", "pixelate", "--quality", "low", "--backend", "software", "--out-dir"])
        .arg(&out_dir)
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "a failed input must give a failing exit status:\n{log}");
    assert_eq!(out.status.code(), Some(1));
    // the inputs around the broken ones were rendered
    let names = listing(&out_dir);
    assert_eq!(names.len(), 2, "{names:?}\n{log}");
    assert!(names.iter().any(|n| n.starts_with("one-phthora-")) && names.iter().any(|n| n.starts_with("two-phthora-")), "{names:?}");
    // the error list names every failure, the progress lines say where it happened
    assert!(log.contains("2 of 4 inputs failed") && log.contains("broken.mp4") && log.contains("missing.mp4"), "{log}");
    assert!(log.contains("[2/4] FAILED") && log.contains("[4/4] wrote"), "{log}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn batch_arguments_are_checked() {
    let d = tmp_dir("batch-args");
    let dir = d.to_str().unwrap();
    let err = |args: &[&str]| {
        let out = bin().arg("render").args(args).output().unwrap();
        assert!(!out.status.success(), "{args:?} was accepted");
        String::from_utf8_lossy(&out.stderr).to_string()
    };
    // several inputs need a folder, and --out names one file
    assert!(err(&["a.mp4", "b.mp4", "--effect", "pixelate"]).contains("--out-dir"));
    assert!(err(&["a.mp4", "b.mp4", "--out", "x.mp4"]).contains("--out-dir"));
    assert!(err(&["a.mp4", "--out", "x.mp4", "--out-dir", dir]).contains("--out-dir"));
    assert!(err(&["--out-dir", dir]).contains("at least one input"));
    assert!(err(&["a.mp4", "--format", "mp4", "--out", "x.mp4"]).contains("--format"));
    assert!(err(&["a.mp4", "b.mp4", "--format", "mov", "--out-dir", dir]).contains("unknown --format"));
    // nothing was written for rejected arguments
    assert!(listing(&d).is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

/// A beat at 128 BPM: a decaying 60 Hz thump on every beat.
fn click_track(d: &Path) -> PathBuf {
    let p = d.join("beat.wav");
    let st = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "aevalsrc=0.8*sin(2*PI*60*t)*exp(-mod(t-0.2\\,0.46875)*25)*gte(t\\,0.2):s=44100:d=20"])
        .arg(&p)
        .status()
        .unwrap();
    assert!(st.success());
    p
}

fn seconds(path: &Path, stream: &str) -> f64 {
    let out = Command::new("ffprobe").args(["-v", "error", "-select_streams", stream, "-show_entries", "stream=duration", "-of", "csv=p=0"]).arg(path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

#[test]
fn bars_of_a_detected_tempo_set_the_length() {
    if !have("ffmpeg") {
        eprintln!("skipped: ffmpeg not found");
        return;
    }
    let d = tmp_dir("bars");
    let beat = click_track(&d);
    let out = d.join("loop.mp4");
    run(bin().args(["render", "--effect", "rgb_shift", "--size", "64x64", "--fps", "24", "--bpm", "auto", "--bars", "2", "--loop-start", "bar:1", "--audio"]).arg(&beat).arg("--out").arg(&out));
    // 2 bars at 128 BPM: 3.75 s, 90 frames
    assert!(probe(&out, "stream=nb_read_frames").contains("nb_read_frames=90"), "{}", probe(&out, "stream=nb_read_frames"));
    assert!((seconds(&out, "a:0") - 3.75).abs() < 0.05, "{}", seconds(&out, "a:0"));
    // without audio there is no tempo to detect
    let o = bin().args(["render", "--effect", "rgb_shift", "--bpm", "auto", "--bars", "2", "--out"]).arg(d.join("x.mp4")).output().unwrap();
    assert!(!o.status.success() && String::from_utf8_lossy(&o.stderr).contains("needs audio"), "{}", String::from_utf8_lossy(&o.stderr));
    let o = bin().args(["render", "--effect", "rgb_shift", "--bars", "2", "--out"]).arg(d.join("x.mp4")).output().unwrap();
    assert!(!o.status.success() && String::from_utf8_lossy(&o.stderr).contains("--bars needs"));
    let _ = std::fs::remove_dir_all(&d);
}
