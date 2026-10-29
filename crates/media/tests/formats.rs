//! Output formats of the encoder: animated WebP, APNG, PNG sequence, stills and the GIF recipe.
//! Files are written by the encoder and inspected with ffprobe / ffmpeg.

use std::path::{Path, PathBuf};
use std::process::Command;

use phthora_media::*;

struct Tmp(PathBuf);

impl Tmp {
    fn new(name: &str) -> Tmp {
        let dir = std::env::temp_dir().join(format!("phthora-formats-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    fn p(&self, f: &str) -> PathBuf {
        self.0.join(f)
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Half opaque, half transparent, red channel gradient, blue = frame index.
fn alpha_frame(i: u32, w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let a = if x < w / 2 { 255 } else { 0 };
            v.extend_from_slice(&[(x * 3) as u8, (y * 4) as u8, (i * 8) as u8, a]);
        }
    }
    v
}

fn opaque_frame(i: u32, w: u32, h: u32) -> Vec<u8> {
    let mut f = alpha_frame(i, w, h);
    f.as_chunks_mut::<4>().0.iter_mut().for_each(|p| p[3] = 255);
    f
}

/// Pseudo-random opaque pixels, different in every frame.
fn noise_frame(i: usize, w: usize, h: usize) -> Vec<u8> {
    (0..w * h * 4).map(|k| if k % 4 == 3 { 255 } else { ((k * 7919 + i * 31 + (k / 5) * 13) % 251) as u8 }).collect()
}

/// False (with a note) when this ffmpeg lacks an encoder: the test is skipped.
fn need(formats: &[Format]) -> bool {
    let missing = formats.iter().find_map(|f| missing_encoder(*f));
    if let Some(m) = missing {
        eprintln!("skipped: ffmpeg has no {m} encoder");
    }
    ffmpeg_available() && missing.is_none()
}

fn px(data: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let o = ((y * w + x) * 4) as usize;
    data[o..o + 4].try_into().unwrap()
}

fn settings(t: &Tmp, name: &str, format: Format, frames: u32) -> EncodeSettings {
    EncodeSettings::new(t.p(name), format, 64, 48, 25.0, frames as f64 / 25.0)
}

fn encode(s: EncodeSettings, frames: u32, mk: impl Fn(u32) -> Vec<u8>) -> PathBuf {
    let mut e = Encoder::start(s).unwrap();
    for i in 0..frames {
        e.push_frame(&mk(i)).unwrap();
    }
    assert_eq!(e.frames_written(), frames as u64);
    e.finish().unwrap()
}

/// ffprobe fields of the first video stream, e.g. `codec_name=webp_anim`, with counted frames.
fn stream_info(path: &Path) -> String {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-count_frames", "-select_streams", "v:0", "-show_entries", "stream=codec_name,pix_fmt,width,height,nb_read_frames", "-of", "default=nw=1"])
        .arg(path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// All frames of `path` decoded to RGBA by ffmpeg.
fn decode_rgba(path: &Path) -> Vec<u8> {
    let out = Command::new("ffmpeg").args(["-v", "error", "-i"]).arg(path).args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    out.stdout
}

/// pix_fmt of the stream has an alpha channel (`yuva420p`, `rgba`, `argb` ...).
fn has_alpha_pix_fmt(info: &str) -> bool {
    info.lines().find_map(|l| l.strip_prefix("pix_fmt=")).is_some_and(|p| p.contains('a'))
}

#[test]
fn animated_webp_and_apng_keep_alpha() {
    if !need(&[Format::Webp, Format::Apng]) {
        return;
    }
    for (name, format, codec) in [("a.webp", Format::Webp, "codec_name=webp_anim"), ("a.apng", Format::Apng, "codec_name=apng")] {
        let t = Tmp::new(name);
        let p = encode(settings(&t, name, format, 10), 10, |i| alpha_frame(i, 64, 48));
        let info = stream_info(&p);
        assert!(info.contains(codec) && info.contains("width=64") && info.contains("height=48"), "{name}: {info}");
        assert!(info.contains("nb_read_frames=10"), "{name}: {info}");
        assert!(has_alpha_pix_fmt(&info), "{name}: {info}");
        let all = decode_rgba(&p);
        assert_eq!(all.len(), 64 * 48 * 4 * 10, "{name}");
        let last = &all[64 * 48 * 4 * 9..];
        assert!(px(last, 64, 8, 24)[3] > 240, "{name}");
        assert!(px(last, 64, 56, 24)[3] < 15, "{name}");
    }
}

#[test]
fn png_sequence_to_folder_and_pattern() {
    if !need(&[Format::PngSeq]) {
        return;
    }
    let t = Tmp::new("pngseq");
    // folder: frames land inside as <folder>-00001.png ...
    let dir = t.p("frames");
    let p = encode(settings(&t, "frames", Format::PngSeq, 6), 6, |i| alpha_frame(i, 64, 48));
    assert_eq!(p, dir);
    let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    assert_eq!(names, (1..=6).map(|n| format!("frames-{n:05}.png")).collect::<Vec<_>>());
    let info = stream_info(&dir.join("frames-00001.png"));
    assert!(info.contains("codec_name=png") && info.contains("width=64") && info.contains("height=48") && has_alpha_pix_fmt(&info), "{info}");
    let img = load_image(&dir.join("frames-00006.png"), 4096).unwrap();
    assert_eq!(px(&img.data, 64, 5, 2), [15, 8, 40, 255]);
    assert_eq!(px(&img.data, 64, 60, 10)[3], 0);

    // a shorter export into the same folder leaves no stale frames; other files stay
    std::fs::write(dir.join("notes.txt"), "keep").unwrap();
    encode(settings(&t, "frames", Format::PngSeq, 2), 2, |i| alpha_frame(i, 64, 48));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 3);
    assert!(!dir.join("frames-00003.png").exists() && dir.join("notes.txt").exists());

    // explicit pattern in a folder that does not exist yet
    let p = encode(settings(&t, "deep/dir/shot_%05d.png", Format::PngSeq, 3), 3, |i| opaque_frame(i, 64, 48));
    assert_eq!(p, t.p("deep/dir/shot_%05d.png"));
    assert!(t.p("deep/dir/shot_00003.png").exists() && !t.p("deep/dir/shot_00004.png").exists());

    // cancel removes the frames and the then empty folder
    let mut e = Encoder::start(settings(&t, "gone", Format::PngSeq, 20)).unwrap();
    for i in 0..3 {
        e.push_frame(&opaque_frame(i, 64, 48)).unwrap();
    }
    e.cancel();
    assert!(!t.p("gone").exists());
}

#[test]
fn jpg_and_webp_stills() {
    if !need(&[Format::Jpg, Format::WebpStill]) {
        return;
    }
    let t = Tmp::new("stills");
    let p = encode(settings(&t, "a.jpg", Format::Jpg, 1), 1, |i| opaque_frame(i, 64, 48));
    let info = stream_info(&p);
    assert!(info.contains("codec_name=mjpeg") && info.contains("width=64") && info.contains("height=48") && info.contains("nb_read_frames=1"), "{info}");
    assert!(!has_alpha_pix_fmt(&info), "{info}");
    let img = load_image(&p, 4096).unwrap();
    let got = px(&img.data, 64, 20, 10);
    for (g, w) in got.iter().zip([60i32, 40, 0, 255]) {
        assert!((*g as i32 - w).abs() <= 12, "jpg pixel {got:?}");
    }
    // quality steps change the size
    let size = |q: Quality| {
        let mut s = settings(&t, &format!("q{q:?}.jpg"), Format::Jpg, 1);
        s.quality = q;
        std::fs::metadata(encode(s, 1, |_| noise_frame(0, 64, 48))).unwrap().len()
    };
    let (u, m, l) = (size(Quality::Ultra), size(Quality::Medium), size(Quality::Low));
    assert!(u > m && m > l, "{u} {m} {l}");

    let p = encode(settings(&t, "a.webp", Format::WebpStill, 1), 1, |i| alpha_frame(i, 64, 48));
    let info = stream_info(&p);
    assert!(info.contains("codec_name=webp") && info.contains("nb_read_frames=1") && has_alpha_pix_fmt(&info), "{info}");
    let rgba = decode_rgba(&p);
    assert!(px(&rgba, 64, 8, 24)[3] > 240 && px(&rgba, 64, 56, 24)[3] < 15);

    // stills take one frame
    let mut e = Encoder::start(settings(&t, "b.jpg", Format::Jpg, 1)).unwrap();
    e.push_frame(&opaque_frame(0, 64, 48)).unwrap();
    assert!(e.push_frame(&opaque_frame(0, 64, 48)).is_err());
}

#[test]
fn webp_quality_steps_change_the_size() {
    if !need(&[Format::Webp]) {
        return;
    }
    let t = Tmp::new("webpq");
    let size = |q: Quality| {
        let mut s = settings(&t, &format!("{q:?}.webp"), Format::Webp, 6);
        s.quality = q;
        std::fs::metadata(encode(s, 6, |i| noise_frame(i as usize, 64, 48))).unwrap().len()
    };
    let (u, l) = (size(Quality::Ultra), size(Quality::Low));
    assert!(u > l, "{u} {l}");
}

#[test]
fn gif_recipe_palette_alpha_and_fps_cap() {
    if !need(&[Format::Gif]) {
        return;
    }
    let t = Tmp::new("gif_recipe");
    // colors per quality step, counted in the decoded first frame
    let colors = |q: Quality| {
        let mut s = settings(&t, &format!("{q:?}.gif"), Format::Gif, 4);
        s.quality = q;
        let p = encode(s, 4, |i| noise_frame(i as usize, 64, 48));
        let rgba = decode_rgba(&p);
        let first: std::collections::HashSet<[u8; 4]> = rgba[..64 * 48 * 4].as_chunks::<4>().0.iter().copied().collect();
        first.len()
    };
    let (hi, med, low) = (colors(Quality::High), colors(Quality::Medium), colors(Quality::Low));
    assert!(hi <= 256 && hi > 128 && med <= 128 && med > 64 && low <= 64, "{hi} {med} {low}");

    // transparency survives the recipe
    let p = encode(settings(&t, "alpha.gif", Format::Gif, 6), 6, |i| alpha_frame(i, 64, 48));
    let rgba = decode_rgba(&p);
    let last = &rgba[64 * 48 * 4 * 5..];
    assert!(px(last, 64, 8, 24)[3] > 240 && px(last, 64, 56, 24)[3] < 15);

    // frame delays are hundredths of a second: more than 50 fps is refused, 30 fps keeps its length
    let mut s = settings(&t, "fast.gif", Format::Gif, 10);
    s.fps = 60.0;
    let err = Encoder::start(s).err().expect("60 fps gif must be refused");
    assert!(err.to_string().contains("50 fps"), "{err}");
    let mut s = settings(&t, "thirty.gif", Format::Gif, 30);
    s.fps = 30.0;
    let p = encode(s, 30, |i| opaque_frame(i, 64, 48));
    let info = probe(&p).unwrap();
    assert!((info.duration - 1.0).abs() < 0.05, "30 frames at 30 fps last {} s", info.duration);
    assert_eq!(count_frames(&p).unwrap(), 30);
}

#[test]
fn format_availability_matches_the_encoder_list() {
    assert!(ffmpeg_available());
    for f in Format::STILLS.iter().chain(Format::ANIMATED.iter()) {
        assert_eq!(format_available(*f), missing_encoder(*f).is_none(), "{f:?}");
        assert_eq!(Format::from_id(f.id()), Some(*f));
    }
    assert!(format_available(Format::Png) && format_available(Format::Gif));
}
