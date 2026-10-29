use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use phthora_media::*;

struct Tmp(PathBuf);

impl Tmp {
    fn new(name: &str) -> Tmp {
        let dir = std::env::temp_dir().join(format!("phthora-media-test-{}-{name}", std::process::id()));
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

fn ff(args: &[&str]) {
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-y"])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(out.status.success(), "ffmpeg {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// 64x48, 25 fps, 2 s testsrc2 with a sine track.
fn make_video(t: &Tmp) -> PathBuf {
    let p = t.p("src.mp4");
    ff(&[
        "-f", "lavfi", "-i", "testsrc2=s=64x48:r=25:d=2", "-f", "lavfi", "-i", "sine=d=2",
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest", p.to_str().unwrap(),
    ]);
    p
}

/// Feed raw RGBA frames to ffmpeg with the given output args.
fn rgba_to_file(frames: &[Vec<u8>], w: u32, h: u32, out_args: &[&str], out: &Path) {
    let mut child = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{w}x{h}"), "-framerate", "25", "-i", "-"])
        .args(out_args)
        .arg(out)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut si = child.stdin.take().unwrap();
        for f in frames {
            si.write_all(f).unwrap();
        }
    }
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
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

fn px(data: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let o = ((y * w + x) * 4) as usize;
    data[o..o + 4].try_into().unwrap()
}

fn drain(mut d: Decoder) -> Vec<Frame> {
    let mut v = Vec::new();
    while let Some(f) = d.next_frame() {
        v.push(f);
    }
    assert!(d.error().is_none(), "decoder error: {:?}", d.error());
    v
}

fn open(path: &Path, w: u32, h: u32, opts: DecodeOptions) -> Decoder {
    let info = probe(path).unwrap();
    Decoder::open(path, &info, w, h, opts).unwrap()
}

#[test]
fn ffmpeg_is_available() {
    assert!(ffmpeg_available());
}

#[test]
fn probe_video() {
    let t = Tmp::new("probe_video");
    let info = probe(&make_video(&t)).unwrap();
    assert_eq!(info.kind, MediaKind::Video);
    assert_eq!((info.width, info.height), (64, 48));
    assert_eq!(info.rotation, 0);
    assert!((info.fps - 25.0).abs() < 1e-9);
    assert_eq!(info.frame_count, 50);
    assert!((info.duration - 2.0).abs() < 0.05, "{}", info.duration);
    assert!(info.has_audio);
    assert_eq!(info.codec, "h264");
    assert!(!info.vfr);
    assert_eq!(count_frames(&t.p("src.mp4")).unwrap(), 50);
}

#[test]
fn probe_image_and_gif() {
    let t = Tmp::new("probe_image");
    rgba_to_file(&[alpha_frame(0, 40, 30)], 40, 30, &["-frames:v", "1"], &t.p("a.png"));
    let info = probe(&t.p("a.png")).unwrap();
    assert_eq!(info.kind, MediaKind::Image);
    assert_eq!((info.width, info.height), (40, 30));
    assert_eq!((info.duration, info.frame_count, info.has_audio), (0.0, 1, false));

    let frames: Vec<_> = (0..10).map(|i| opaque_frame(i, 40, 30)).collect();
    rgba_to_file(&frames, 40, 30, &[], &t.p("a.gif"));
    let info = probe(&t.p("a.gif")).unwrap();
    assert_eq!(info.kind, MediaKind::Gif);
    assert_eq!((info.width, info.height), (40, 30));
    assert!(info.fps > 0.0);
    assert_eq!(count_frames(&t.p("a.gif")).unwrap(), 10);

    assert!(probe(&t.p("missing.mp4")).is_err());
    std::fs::write(t.p("junk.mp4"), b"not media").unwrap();
    assert!(probe(&t.p("junk.mp4")).is_err());
}

#[test]
fn probe_rotation_swaps_size() {
    let t = Tmp::new("probe_rotation");
    let src = make_video(&t);
    let out = t.p("rot.mp4");
    ff(&["-display_rotation", "90", "-i", src.to_str().unwrap(), "-c", "copy", out.to_str().unwrap()]);
    let info = probe(&out).unwrap();
    assert_eq!((info.width, info.height), (48, 64));
    assert!(info.rotation == 90 || info.rotation == 270, "{}", info.rotation);

    // Decoder output must already be upright (ffmpeg autorotate).
    let frames = drain(open(&out, 48, 64, DecodeOptions::default()));
    assert_eq!(frames.len(), 50);
    assert_eq!((frames[0].width, frames[0].height), (48, 64));
}

#[test]
fn probe_exif_orientation() {
    let t = Tmp::new("probe_exif");
    let plain = t.p("plain.jpg");
    ff(&["-f", "lavfi", "-i", "testsrc2=s=64x32:d=1", "-frames:v", "1", plain.to_str().unwrap()]);
    let jpg = std::fs::read(&plain).unwrap();
    // Minimal big-endian EXIF block with Orientation = 6 (rotate 90 cw).
    let mut exif: Vec<u8> = b"Exif\0\0MM\0*\0\0\0\x08".to_vec();
    exif.extend_from_slice(&[0, 1, 0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 6, 0, 0, 0, 0, 0, 0]);
    let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
    out.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(&exif);
    out.extend_from_slice(&jpg[2..]);
    std::fs::write(t.p("rot.jpg"), out).unwrap();

    let info = probe(&t.p("rot.jpg")).unwrap();
    assert_eq!(info.kind, MediaKind::Image);
    assert_eq!((info.width, info.height), (32, 64), "rotation {}", info.rotation);
    assert_eq!(info.rotation, 90);
    let img = load_image(&t.p("rot.jpg"), 4096).unwrap();
    assert_eq!((img.width, img.height), (32, 64));
    assert_eq!(img.data.len(), 32 * 64 * 4);
}

#[test]
fn load_image_pixels_alpha_and_limit() {
    let t = Tmp::new("load_image");
    rgba_to_file(&[alpha_frame(3, 40, 30)], 40, 30, &["-frames:v", "1"], &t.p("a.png"));
    let img = load_image(&t.p("a.png"), 4096).unwrap();
    assert_eq!((img.width, img.height), (40, 30));
    assert_eq!(img.data.len(), 40 * 30 * 4);
    // Top-down, straight alpha: opaque left half, transparent right half.
    assert_eq!(px(&img.data, 40, 5, 2), [15, 8, 24, 255]);
    assert_eq!(px(&img.data, 40, 10, 29), [30, 116, 24, 255]);
    assert_eq!(px(&img.data, 40, 30, 10)[3], 0);

    let small = load_image(&t.p("a.png"), 20).unwrap();
    assert_eq!((small.width, small.height), (20, 14));
    assert_eq!(small.data.len(), 20 * 14 * 4);
    assert!(load_image(&t.p("nope.png"), 100).is_err());
}

#[test]
fn decoder_native_scale_and_start() {
    let t = Tmp::new("dec_native");
    let src = make_video(&t);
    let frames = drain(open(&src, 64, 48, DecodeOptions::default()));
    assert_eq!(frames.len(), 50);
    assert_eq!(frames[0].data.len(), 64 * 48 * 4);
    assert_eq!(frames[7].index, 7);
    assert!((frames[25].pts - 1.0).abs() < 1e-9);
    assert!(frames.iter().all(|f| f.pass == 0 && f.data.as_chunks::<4>().0.iter().all(|p| p[3] == 255)));

    let scaled = drain(open(&src, 32, 24, DecodeOptions::default()));
    assert_eq!(scaled.len(), 50);
    assert_eq!((scaled[0].width, scaled[0].height, scaled[0].data.len()), (32, 24, 32 * 24 * 4));

    let part = drain(open(&src, 64, 48, DecodeOptions { start: 0.5, end: Some(1.5), ..Default::default() }));
    // Frame boundaries after a keyframe-seek can add or drop one frame.
    assert!((24..=25).contains(&part.len()), "{}", part.len());
    assert!((part[0].pts - 0.5).abs() < 1e-9);
    // First frame at 0.5 s equals frame 12/13 of the full decode.
    let p0 = &part[0].data;
    assert!(p0 == &frames[12].data || p0 == &frames[13].data);
    assert!((part[0].pts - 0.5).abs() < 1e-9);
}

#[test]
fn decoder_exact_frame_count_with_fps() {
    let t = Tmp::new("dec_fps");
    let src = make_video(&t);
    let cases: &[(f64, f64, Option<f64>, usize)] = &[
        (10.0, 0.0, None, 20),
        (60.0, 0.0, None, 120),
        (29.97, 0.0, None, 60),
        (24.0, 0.5, Some(1.5), 24),
        (30.0, 0.3, Some(1.9), 48),
        // Past the end of the file: last frame is repeated.
        (10.0, 0.0, Some(3.0), 30),
        // Shorter than a frame.
        (25.0, 0.0, Some(0.01), 1),
    ];
    for &(fps, start, end, expect) in cases {
        let frames = drain(open(&src, 64, 48, DecodeOptions { start, end, fps: Some(fps), ..Default::default() }));
        assert_eq!(frames.len(), expect, "fps {fps} start {start} end {end:?}");
        assert!(frames.iter().enumerate().all(|(i, f)| f.index == i as u64));
    }
    // Repeated tail frames are identical to the last real frame.
    let f = drain(open(&src, 64, 48, DecodeOptions { fps: Some(10.0), end: Some(3.0), ..Default::default() }));
    assert_eq!(f[29].data, f[20].data);
    assert_ne!(f[19].data, f[5].data);
}

#[test]
fn decoder_loops_and_drops_cleanly() {
    let t = Tmp::new("dec_loop");
    let src = make_video(&t);
    let mut d = open(&src, 64, 48, DecodeOptions { looped: true, ..Default::default() });
    let mut got = Vec::new();
    for _ in 0..130 {
        got.push(d.next_frame().expect("looped decoder ended"));
    }
    assert_eq!(got[49].pass, 0);
    assert_eq!((got[50].pass, got[50].index), (1, 0));
    assert_eq!(got[129].pass, 2);
    assert_eq!(got[50].data, got[0].data);
    let t0 = Instant::now();
    drop(d);
    assert!(t0.elapsed() < Duration::from_secs(3), "drop took {:?}", t0.elapsed());

    // Looped + fps: fixed frames per pass.
    let mut d = open(&src, 64, 48, DecodeOptions { looped: true, fps: Some(10.0), end: Some(1.0), ..Default::default() });
    let v: Vec<_> = (0..25).map(|_| d.next_frame().unwrap()).collect();
    assert_eq!((v[9].pass, v[10].pass, v[10].index, v[24].pass), (0, 1, 0, 2));
}

#[test]
fn decoder_try_next_frame_does_not_block() {
    let t = Tmp::new("dec_try");
    let src = make_video(&t);
    let mut d = open(&src, 64, 48, DecodeOptions::default());
    let mut n = 0;
    let deadline = Instant::now() + Duration::from_secs(20);
    while !d.is_finished() {
        assert!(Instant::now() < deadline, "timeout");
        match d.try_next_frame() {
            Some(_) => n += 1,
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    }
    assert_eq!(n, 50);
}

#[test]
fn decoder_reports_errors() {
    let t = Tmp::new("dec_err");
    std::fs::write(t.p("junk.mp4"), b"not media").unwrap();
    let info = probe(&make_video(&t)).unwrap();
    let mut d = Decoder::open(&t.p("junk.mp4"), &info, 64, 48, DecodeOptions::default()).unwrap();
    assert!(d.next_frame().is_none());
    assert!(d.error().is_some());
    // Looped decoders must not spin on a broken file.
    let mut d = Decoder::open(&t.p("junk.mp4"), &info, 64, 48, DecodeOptions { looped: true, ..Default::default() }).unwrap();
    assert!(d.next_frame().is_none());
}

#[test]
fn decoder_image_input() {
    let t = Tmp::new("dec_img");
    rgba_to_file(&[alpha_frame(0, 40, 30)], 40, 30, &["-frames:v", "1"], &t.p("a.png"));
    let one = drain(open(&t.p("a.png"), 40, 30, DecodeOptions::default()));
    assert_eq!(one.len(), 1);
    let many = drain(open(&t.p("a.png"), 20, 14, DecodeOptions { fps: Some(10.0), end: Some(1.0), ..Default::default() }));
    assert_eq!(many.len(), 10);
    assert_eq!(many[9].data.len(), 20 * 14 * 4);
}

fn make_alpha_webm(t: &Tmp) -> PathBuf {
    let frames: Vec<_> = (0..10).map(|i| alpha_frame(i, 64, 48)).collect();
    let out = t.p("alpha.webm");
    rgba_to_file(
        &frames, 64, 48,
        &["-c:v", "libvpx-vp9", "-pix_fmt", "yuva420p", "-auto-alt-ref", "0", "-crf", "20", "-b:v", "0"],
        &out,
    );
    out
}

#[test]
fn decoder_keeps_vp9_alpha() {
    let t = Tmp::new("dec_alpha");
    let webm = make_alpha_webm(&t);
    let info = probe(&webm).unwrap();
    assert_eq!(info.codec, "vp9");
    let frames = drain(open(&webm, 64, 48, DecodeOptions::default()));
    assert_eq!(frames.len(), 10);
    assert!(px(&frames[3].data, 64, 8, 24)[3] > 240);
    assert!(px(&frames[3].data, 64, 56, 24)[3] < 15);
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

#[test]
fn encode_mp4_with_and_without_audio() {
    let t = Tmp::new("enc_mp4");
    let p = encode(settings(&t, "a.mp4", Format::Mp4, 25), 25, |i| opaque_frame(i, 64, 48));
    let info = probe(&p).unwrap();
    assert_eq!((info.kind, info.width, info.height), (MediaKind::Video, 64, 48));
    assert_eq!(info.codec, "h264");
    assert!((info.fps - 25.0).abs() < 1e-9);
    assert_eq!(info.frame_count, 25);
    assert_eq!(count_frames(&p).unwrap(), 25);
    assert!(!info.has_audio);

    // Audio shorter than the video (0.5 s vs 1.6 s) gets looped; trim applies.
    let wav = t.p("tone.wav");
    ff(&["-f", "lavfi", "-i", "sine=d=0.5", wav.to_str().unwrap()]);
    let mut s = settings(&t, "b.mp4", Format::Mp4, 40);
    s.audio = Some(AudioSource { path: wav.clone(), start: 0.1, end: Some(0.4) });
    let p = encode(s, 40, |i| opaque_frame(i, 64, 48));
    let info = probe(&p).unwrap();
    assert!(info.has_audio);
    assert_eq!(count_frames(&p).unwrap(), 40);
    assert!((info.duration - 1.6).abs() < 0.05, "video duration {}", info.duration);
    let raw = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", p.to_str().unwrap()])
        .output()
        .unwrap();
    let total: f64 = String::from_utf8_lossy(&raw.stdout).trim().parse().unwrap();
    assert!((total - 1.6).abs() < 0.1, "container duration {total}");

    // Audio from a video file works too.
    let mut s = settings(&t, "c.mp4", Format::Mp4, 10);
    s.audio = Some(AudioSource { path: make_video(&t), start: 0.0, end: None });
    assert!(probe(&encode(s, 10, |i| opaque_frame(i, 64, 48))).unwrap().has_audio);

    // Source without audio is rejected up front.
    let mut s = settings(&t, "d.mp4", Format::Mp4, 1);
    s.audio = Some(AudioSource { path: p.with_file_name("a.mp4"), start: 0.0, end: None });
    assert!(matches!(Encoder::start(s), Err(MediaError::InvalidInput(_))));
}

#[test]
fn encode_mp4_colors_survive_roundtrip() {
    let t = Tmp::new("enc_color");
    let flat = |_: u32| [200u8, 100, 50, 255].repeat(64 * 48);
    let mut s = settings(&t, "c.mp4", Format::Mp4, 5);
    s.quality = Quality::Ultra;
    let p = encode(s, 5, flat);
    let frames = drain(open(&p, 64, 48, DecodeOptions::default()));
    let got = px(&frames[2].data, 64, 30, 20);
    for (g, w) in got.iter().zip([200i32, 100, 50, 255]) {
        assert!((*g as i32 - w).abs() <= 3, "got {got:?}");
    }
}

#[test]
fn encode_webm_with_alpha_and_audio() {
    let t = Tmp::new("enc_webm");
    let wav = t.p("tone.wav");
    ff(&["-f", "lavfi", "-i", "sine=d=2", wav.to_str().unwrap()]);
    let mut s = settings(&t, "a.webm", Format::WebM, 12);
    s.audio = Some(AudioSource { path: wav, start: 0.0, end: None });
    let p = encode(s, 12, |i| alpha_frame(i, 64, 48));
    let info = probe(&p).unwrap();
    assert_eq!((info.codec.as_str(), info.width, info.height), ("vp9", 64, 48));
    assert!(info.has_audio);
    assert_eq!(count_frames(&p).unwrap(), 12);
    let frames = drain(open(&p, 64, 48, DecodeOptions::default()));
    assert_eq!(frames.len(), 12);
    assert!(px(&frames[5].data, 64, 8, 24)[3] > 240);
    assert!(px(&frames[5].data, 64, 56, 24)[3] < 15);
}

#[test]
fn encode_gif_and_png() {
    let t = Tmp::new("enc_gif");
    let wav = t.p("tone.wav");
    ff(&["-f", "lavfi", "-i", "sine=d=1", wav.to_str().unwrap()]);
    let mut s = settings(&t, "a.gif", Format::Gif, 25);
    s.audio = Some(AudioSource { path: wav, start: 0.0, end: None });
    let p = encode(s, 25, |i| opaque_frame(i, 64, 48));
    let info = probe(&p).unwrap();
    assert_eq!((info.kind, info.width, info.height), (MediaKind::Gif, 64, 48));
    assert!(!info.has_audio);
    assert_eq!(count_frames(&p).unwrap(), 25);
    assert!((info.duration - 1.0).abs() < 0.1, "{}", info.duration);

    let p = encode(settings(&t, "a.png", Format::Png, 1), 1, |i| alpha_frame(i, 64, 48));
    let img = load_image(&p, 4096).unwrap();
    assert_eq!((img.width, img.height), (64, 48));
    assert_eq!(px(&img.data, 64, 5, 2), [15, 8, 0, 255]);
    assert_eq!(px(&img.data, 64, 60, 10)[3], 0);

    // PNG takes a single frame.
    let mut e = Encoder::start(settings(&t, "b.png", Format::Png, 1)).unwrap();
    e.push_frame(&alpha_frame(0, 64, 48)).unwrap();
    assert!(e.push_frame(&alpha_frame(0, 64, 48)).is_err());
}

#[test]
fn encode_cancel_and_errors() {
    let t = Tmp::new("enc_cancel");
    let mut e = Encoder::start(settings(&t, "x.mp4", Format::Mp4, 50)).unwrap();
    for i in 0..5 {
        e.push_frame(&opaque_frame(i, 64, 48)).unwrap();
    }
    e.cancel();
    assert!(!t.p("x.mp4").exists());

    // Dropping without finish() also cleans up.
    let mut e = Encoder::start(settings(&t, "y.mp4", Format::Mp4, 50)).unwrap();
    e.push_frame(&opaque_frame(0, 64, 48)).unwrap();
    drop(e);
    assert!(!t.p("y.mp4").exists());

    let mut e = Encoder::start(settings(&t, "z.mp4", Format::Mp4, 1)).unwrap();
    assert!(matches!(e.push_frame(&[0u8; 10]), Err(MediaError::InvalidInput(_))));
    e.cancel();

    let mut odd = settings(&t, "odd.mp4", Format::Mp4, 1);
    odd.width = 63;
    assert!(matches!(Encoder::start(odd), Err(MediaError::InvalidInput(_))));
    let mut bad_fps = settings(&t, "f.mp4", Format::Mp4, 1);
    bad_fps.fps = 0.0;
    assert!(Encoder::start(bad_fps).is_err());

    // A run that ffmpeg itself rejects (output path is a directory) returns its stderr.
    std::fs::create_dir(t.p("isdir.mp4")).unwrap();
    let mut e = Encoder::start(settings(&t, "isdir.mp4", Format::Mp4, 1)).unwrap();
    let mut err = None;
    for i in 0..200 {
        if let Err(e2) = e.push_frame(&opaque_frame(i, 64, 48)) {
            err = Some(e2);
            break;
        }
    }
    match err {
        Some(MediaError::Ffmpeg { stderr, .. }) => assert!(!stderr.is_empty()),
        None => assert!(matches!(e.finish(), Err(MediaError::Ffmpeg { .. }))),
        Some(other) => panic!("unexpected error {other:?}"),
    }
}

#[test]
fn encode_is_deterministic_when_asked() {
    let t = Tmp::new("enc_det");
    let run = |name: &str, det: bool| {
        let mut s = settings(&t, name, Format::Mp4, 30);
        s.deterministic = det;
        std::fs::read(encode(s, 30, |i| opaque_frame(i, 64, 48))).unwrap()
    };
    let (a, b) = (run("d1.mp4", true), run("d2.mp4", true));
    assert_eq!(a, b, "deterministic mode must give identical bytes");
    // Default mode: x264 output is normally identical too, but not guaranteed.
    let (c, d) = (run("n1.mp4", false), run("n2.mp4", false));
    eprintln!("default mode identical: {}", c == d);
}

#[test]
fn encode_vaapi_if_available() {
    if !vaapi_available() {
        eprintln!("VAAPI not available, skipped");
        return;
    }
    let t = Tmp::new("enc_vaapi");
    // Hardware minimum is 96x32.
    let mut s = EncodeSettings::new(t.p("v.mp4"), Format::Mp4, 128, 96, 25.0, 0.8);
    s.backend = Backend::Vaapi;
    let p = encode(s, 20, |i| opaque_frame(i, 128, 96));
    let info = probe(&p).unwrap();
    assert_eq!((info.codec.as_str(), info.width, info.height), ("h264", 128, 96));
    assert_eq!(count_frames(&p).unwrap(), 20);
}

// ---------------------------------------------------------------- clip reader

#[test]
fn clip_reader_random_access_matches_sequential_decode() {
    let t = Tmp::new("clip");
    let src = make_video(&t);
    let info = probe(&src).unwrap();
    let reference: Vec<Vec<u8>> = drain(open(&src, 32, 24, DecodeOptions { fps: Some(25.0), ..Default::default() })).into_iter().map(|f| f.data).collect();
    assert_eq!(reference.len(), 50);
    let clip = ClipReader::open(&src, info, 32, 24);
    assert_eq!(clip.frame_count(), 50);
    // forward, a jump back, reverse run, ping-pong turn at the end, a far jump
    let order: Vec<u64> = (0..12).chain([3]).chain((20..30).rev()).chain([47, 48, 49, 48, 47]).chain([5, 44]).collect();
    for (k, &i) in order.iter().enumerate() {
        let ahead: Vec<u64> = order[k + 1..].iter().take(8).copied().collect();
        let f = clip.get_blocking(i, &ahead).unwrap_or_else(|| panic!("frame {i}: {:?}", clip.error()));
        assert!(f.data == reference[i as usize], "frame {i} differs from sequential decode");
    }
    assert!(clip.error().is_none());
}

#[test]
fn clip_reader_prefetches_without_blocking() {
    let t = Tmp::new("clip-pre");
    let src = make_video(&t);
    let clip = ClipReader::open(&src, probe(&src).unwrap(), 32, 24);
    clip.want(&(0..10).collect::<Vec<_>>());
    let t0 = Instant::now();
    while clip.get(9).is_none() {
        assert!(t0.elapsed() < Duration::from_secs(10), "prefetch did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
    // get() never blocks, also for frames that are not decoded
    let t1 = Instant::now();
    assert!(clip.get(40).is_none());
    assert!(t1.elapsed() < Duration::from_millis(5));
    // past the real end: the last frame
    assert!(clip.get_blocking(999, &[]).is_some());
}

#[test]
fn decode_audio_mono_average() {
    let t = Tmp::new("pcm");
    let p = t.p("stereo.wav");
    // left: 0.5 constant-ish sine, right: silence -> mono is half the left channel
    ff(&["-f", "lavfi", "-i", "sine=f=441:r=44100:d=0.5", "-af", "pan=stereo|c0=c0|c1=0*c0", "-c:a", "pcm_f32le", p.to_str().unwrap()]);
    let mono = decode_audio(&p, 44100).unwrap();
    let left = decode_audio(&{
        let q = t.p("left.wav");
        ff(&["-f", "lavfi", "-i", "sine=f=441:r=44100:d=0.5", "-c:a", "pcm_f32le", q.to_str().unwrap()]);
        q
    }, 44100)
    .unwrap();
    assert_eq!(mono.len(), 22050);
    let peak_m = mono.iter().fold(0f32, |a, v| a.max(v.abs()));
    let peak_l = left.iter().fold(0f32, |a, v| a.max(v.abs()));
    assert!((peak_m / peak_l - 0.5).abs() < 0.01, "mono {peak_m} left {peak_l}");
}
