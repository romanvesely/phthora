//! Virtual camera and camera input through a v4l2loopback device: frames written to the
//! loopback come back through the live source, and a recording becomes a playable file.

use phthora_media::{LiveSource, Record, VirtualCam, list_devices, probe};
use std::time::{Duration, Instant};

#[test]
fn loopback_roundtrip_and_recording() {
    let devs = list_devices();
    eprintln!("devices: {devs:?}");
    // the last loopback device, so a first one stays free for the editor
    let Some(dev) = devs.iter().rev().find(|d| d.loopback).map(|d| d.path.clone()) else {
        eprintln!("skipped: no v4l2loopback device");
        return;
    };
    if !phthora_media::ffmpeg_available() {
        eprintln!("skipped: ffmpeg not found");
        return;
    }
    let (w, h) = (320, 240);
    let cam = VirtualCam::open(&dev, w, h, 30.0).expect("virtual camera");
    // a solid orange frame, fed continuously like the preview does
    let frame: Vec<u8> = [240u8, 140, 20, 255].repeat((w * h) as usize);
    let feed = |until: Instant| {
        while Instant::now() < until {
            cam.push(frame.clone());
            std::thread::sleep(Duration::from_millis(33));
        }
    };
    feed(Instant::now() + Duration::from_millis(500));
    // once written to, the loopback device turns into a camera
    assert!(list_devices().iter().any(|d| d.path == dev && d.capture), "loopback offers capture while fed");

    let dir = std::env::temp_dir().join(format!("phthora-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let rec = dir.join("rec.mkv");
    let live = LiveSource::open(&dev, 160, 120, Some(Record { path: rec.clone(), audio: false })).expect("live source");
    let start = Instant::now();
    let mut got = None;
    while start.elapsed() < Duration::from_secs(10) {
        cam.push(frame.clone());
        std::thread::sleep(Duration::from_millis(33));
        if let Some((n, f)) = live.latest()
            && n >= 20 {
                got = Some(f);
                break;
            }
        assert!(live.error().is_none(), "{:?}", live.error());
    }
    let f = got.expect("frames from the loopback");
    assert_eq!((f.width, f.height), (160, 120));
    let px = &f.data[(60 * 160 + 80) * 4..][..4];
    // yuv420p in between: colour within a few levels
    assert!(px[0].abs_diff(240) < 12 && px[1].abs_diff(140) < 12 && px[2].abs_diff(20) < 16, "{px:?}");
    feed(Instant::now() + Duration::from_millis(600));
    // ffmpeg checks for "stop" between frames: keep the camera running meanwhile
    let path = std::thread::scope(|s| {
        s.spawn(|| feed(Instant::now() + Duration::from_millis(1500)));
        live.stop()
    })
    .expect("recording file");
    let info = probe(&path).expect("probe recording");
    assert!(info.duration > 0.5, "recorded {} s", info.duration);
    assert_eq!((info.width, info.height), (w, h), "recorded at the camera's size");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A real camera: set PHTHORA_CAMERA=/dev/videoN to run (the camera turns on briefly).
#[test]
fn real_camera_delivers_frames() {
    let Some(dev) = std::env::var_os("PHTHORA_CAMERA") else {
        eprintln!("skipped: PHTHORA_CAMERA not set");
        return;
    };
    let live = LiveSource::open(std::path::Path::new(&dev), 640, 360, None).expect("camera");
    let start = Instant::now();
    let mut first = None;
    let mut last = 0;
    while start.elapsed() < Duration::from_secs(4) {
        if let Some((n, f)) = live.latest() {
            assert_eq!((f.width, f.height), (640, 360));
            first.get_or_insert((n, Instant::now()));
            last = n;
        }
        assert!(live.error().is_none(), "{:?}", live.error());
        std::thread::sleep(Duration::from_millis(5));
    }
    let (n0, t0) = first.expect("no frame from the camera");
    let fps = (last - n0) as f64 / t0.elapsed().as_secs_f64();
    eprintln!("camera: {fps:.1} fps after start-up");
    assert!(fps > 20.0, "{fps:.1} fps");
}
