//! Mask quality on a real photo. Needs the model and a photo, so it only runs when
//! `PHTHORA_SEG_MODEL` (the .onnx file) and `PHTHORA_SEG_PHOTO` (a PNG) are set:
//! the photo is the CC0 "Smiling woman with a phone" (Wikimedia Commons, rawpixel),
//! its top 1920x1080 rows scaled to 1280x720 (any size with that 16:9 crop works).

use phthora_seg::{Mask, Segmenter};
use std::path::Path;
use std::time::Instant;

pub fn load_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let mut r = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap())).read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size().unwrap()];
    let i = r.next_frame(&mut buf).unwrap();
    let rgba = match i.color_type {
        png::ColorType::Rgba => buf[..i.buffer_size()].to_vec(),
        png::ColorType::Rgb => buf[..i.buffer_size()].chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        c => panic!("unsupported png color type {c:?}"),
    };
    (i.width, i.height, rgba)
}

/// Outline of the person in that photo, in 1920x1080 pixels.
const OUTLINE: [(f32, f32); 29] = [
    (800., 75.), (900., 62.), (1000., 95.), (1070., 180.), (1085., 260.), (1050., 330.), (1015., 415.), (1100., 485.),
    (1190., 560.), (1260., 650.), (1290., 780.), (1285., 950.), (1215., 1080.), (450., 1080.), (440., 950.), (445., 860.),
    (430., 740.), (440., 690.), (570., 665.), (650., 700.), (700., 650.), (790., 590.), (830., 540.), (850., 440.),
    (800., 400.), (760., 330.), (755., 280.), (730., 200.), (740., 150.),
];

fn inside(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut c = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
            c = !c;
        }
    }
    c
}

/// Intersection over union of the mask (>= 128) and the outline.
pub fn iou(mask: &Mask, w: u32, h: u32) -> f32 {
    let m = mask.resize(w, h);
    let (mut inter, mut union) = (0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            let g = inside(&OUTLINE, (x as f32 + 0.5) / w as f32 * 1920.0, (y as f32 + 0.5) / h as f32 * 1080.0);
            let p = m.data[(y * w + x) as usize] >= 128;
            inter += (g && p) as u32;
            union += (g || p) as u32;
        }
    }
    inter as f32 / union as f32
}

#[test]
fn mask_covers_the_person() {
    let (Ok(model), Ok(photo)) = (std::env::var("PHTHORA_SEG_MODEL"), std::env::var("PHTHORA_SEG_PHOTO")) else {
        eprintln!("skipped: set PHTHORA_SEG_MODEL and PHTHORA_SEG_PHOTO");
        return;
    };
    let (w, h, rgba) = load_png(Path::new(&photo));
    let seg = Segmenter::load(Path::new(&model)).expect("model loads");
    let t = Instant::now();
    let mask = seg.run(&rgba, w, h).expect("segmentation");
    eprintln!("first run {:?}", t.elapsed());
    let t = Instant::now();
    let n = 20;
    for _ in 0..n {
        seg.run(&rgba, w, h).unwrap();
    }
    eprintln!("segmentation of {w}x{h}: {:.1} ms per frame", t.elapsed().as_secs_f64() * 1000.0 / n as f64);
    let score = iou(&mask, w, h);
    eprintln!("IoU {score:.3}");
    assert!(score > 0.85, "IoU {score}");
    // background corner and chest: clearly background / person
    let m = mask.resize(w, h);
    let at = |x: f32, y: f32| m.data[((y * h as f32) as u32 * w + (x * w as f32) as u32) as usize];
    assert!(at(0.95, 0.1) < 30, "background top right");
    assert!(at(0.45, 0.8) > 225, "shirt");
}
