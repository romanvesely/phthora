use phthora_core::chain::EffectInstance;
use phthora_core::modulator::LiveInputs;
use phthora_core::registry::registry;
use phthora_core::TimingSettings;
use phthora_gpu::effects::MISSING_UNIFORMS;
use phthora_gpu::{Engine, GpuContext};
use std::collections::HashMap;

pub fn test_image(w: u32, h: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let fy = y as f32 / h as f32;
            let d = ((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt();
            let ring = ((d * 30.0).sin() * 0.5 + 0.5) * 255.0;
            v.extend_from_slice(&[(fx * 255.0) as u8, (fy * 255.0) as u8, ring as u8, 255]);
        }
    }
    v
}

#[test]
fn all_shaders_compile() {
    let ctx = GpuContext::headless().expect("gpu");
    let lib = phthora_gpu::shader::ShaderLibrary::new(&ctx);
    for name in phthora_gpu::shader::shader_names() {
        let _ = lib.get(name);
    }
}

#[test]
fn render_every_effect() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (320, 240);
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &test_image(w, h));
    let ts = TimingSettings::default();
    let t = ts.at_seconds(1.3);
    let base = {
        engine.render(&[], &t);
        engine.read_output()
    };
    // input stage is a straight copy for an opaque image
    assert_eq!(base, test_image(w, h));
    let mut unchanged = vec![];
    for d in registry() {
        let e = EffectInstance::new(d);
        let r = e.resolve(&t, &LiveInputs::default(), &HashMap::new());
        engine.render(std::slice::from_ref(&r), &t);
        let out = engine.read_output();
        if out == base {
            unchanged.push(d.id.clone());
        }
    }
    let missing = MISSING_UNIFORMS.lock().unwrap().clone();
    println!("unchanged with defaults: {unchanged:?}");
    for (id, m) in &missing {
        println!("missing uniforms {id}: {m:?}");
    }
}

/// Visual check: renders every effect for a few frames and writes PNGs.
/// Set PHTHORA_DUMP_DIR (output) and PHTHORA_DUMP_INPUT (a PNG); skipped otherwise.
#[test]
fn dump_effects() {
    let (Ok(dir), Ok(input)) = (std::env::var("PHTHORA_DUMP_DIR"), std::env::var("PHTHORA_DUMP_INPUT")) else {
        return;
    };
    let only = std::env::var("PHTHORA_DUMP_ONLY").unwrap_or_default();
    let (w, h, px) = phthora_gpu::assets::decode_png(&std::fs::read(input).unwrap()).expect("png");
    let ctx = GpuContext::headless().expect("gpu");
    let ts = TimingSettings { fps: 15.0, duration: 2.0, ..Default::default() };
    std::fs::create_dir_all(&dir).unwrap();
    for d in registry() {
        if !only.is_empty() && !only.split(',').any(|o| o == d.id) {
            continue;
        }
        let mut engine = Engine::new(&ctx, w, h);
        engine.set_source_rgba(w, h, &px);
        let mut e = EffectInstance::new(d);
        e.seed = 0.25;
        let frames = 12;
        for f in 0..frames {
            let t = ts.at_frame(f);
            let r = e.resolve(&t, &LiveInputs::default(), &HashMap::new());
            engine.render(std::slice::from_ref(&r), &t);
        }
        let mut out = engine.read_output();
        // un-premultiply for viewing
        for p in out.as_chunks_mut::<4>().0 {
            let a = p[3] as u32;
            if a > 0 && a < 255 {
                for c in &mut p[..3] {
                    *c = ((*c as u32 * 255) / a).min(255) as u8;
                }
            }
        }
        let f = std::fs::File::create(format!("{dir}/{}.png", d.id)).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.write_header().unwrap().write_image_data(&out).unwrap();
    }
}

#[test]
fn readback_ring_matches_cpu_conversion() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (97, 61); // odd width: padded rows
    let mut engine = Engine::new(&ctx, w, h);
    let mut img = test_image(w, h);
    // all alpha values, so every rounding case of the unpremultiply runs
    for (i, p) in img.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        p[3] = (i % 256) as u8;
    }
    engine.set_source_rgba(w, h, &img);
    engine.background = [0.0; 4];
    let mut rb = phthora_gpu::Readback::new(&ctx, &engine.lib, w, h);
    let ts = TimingSettings::default();
    let mut expected = Vec::new();
    let mut got = Vec::new();
    for i in 0..5u64 {
        let e = EffectInstance::by_id("hue_cycle").unwrap();
        let t = ts.at_frame(i * 7);
        let r = e.resolve(&t, &LiveInputs::default(), &HashMap::new());
        engine.render(std::slice::from_ref(&r), &t);
        let mut px = engine.read_output();
        phthora_gpu::context::unpremultiply(&mut px);
        expected.push(px);
        let out = engine.output();
        if let Some(f) = rb.push(out, engine.blank()) {
            got.push(f);
        }
    }
    while let Some(f) = rb.pop() {
        got.push(f);
    }
    assert_eq!(got.len(), expected.len());
    for (i, (a, b)) in got.iter().zip(&expected).enumerate() {
        assert!(a == b, "frame {i}: GPU output pass differs from CPU conversion");
    }
}

#[test]
fn audio_visualizer_draws_the_spectrum() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (180, 100);
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &[0, 0, 0, 255].repeat((w * h) as usize));
    let ts = TimingSettings::default();
    let t = ts.at_frame(0);
    let mut e = EffectInstance::by_id("audio_visualizer").unwrap();
    e.set("mirror", phthora_core::Value::Bool(false));
    e.set("edgeDampening", phthora_core::Value::Float(0.0));
    let r = e.resolve(&t, &LiveInputs::default(), &HashMap::new());
    let lit = |px: &[u8], x0: u32, x1: u32, y0: u32, y1: u32| {
        (y0..y1).flat_map(|y| (x0..x1).map(move |x| (y * w + x) as usize * 4)).filter(|&i| px[i] > 40 || px[i + 1] > 40).count()
    };
    // without audio nothing is drawn
    engine.render(std::slice::from_ref(&r), &t);
    let px = engine.read_output();
    assert_eq!(lit(&px, 0, w, 0, h), 0);
    // loud low frequencies: tall bars on the left, growing from the bottom
    engine.audio_spectrum = (0..512).map(|i| if i < 100 { 0.9 } else { 0.0 }).collect();
    engine.render(std::slice::from_ref(&r), &t);
    let px = engine.read_output();
    let left = lit(&px, 0, w / 8, h / 2, h);
    assert!(left > (w / 8 * h / 2) as usize / 2, "left bars: {left}");
    assert_eq!(lit(&px, w / 2, w, 0, h), 0, "high bars stay empty");
    assert_eq!(lit(&px, 0, w / 8, 0, h / 20), 0, "bars start at the bottom");
    // bottom of a bar is color1 (green)
    let i = ((h - 2) * w + 2) as usize * 4;
    assert!(px[i + 1] > 200 && px[i] < 60, "{:?}", &px[i..i + 4]);
    // radial mode draws around the centre
    e.set("radial", phthora_core::Value::Bool(true));
    let r = e.resolve(&t, &LiveInputs::default(), &HashMap::new());
    engine.render(std::slice::from_ref(&r), &t);
    let px = engine.read_output();
    assert!(lit(&px, 0, w, 0, h) > 100);
    assert_eq!(lit(&px, w / 2 - 5, w / 2 + 5, h / 2 - 5, h / 2 + 5), 0, "inner circle stays empty");
}

#[test]
fn drawing_effects_use_the_canvas() {
    use phthora_gpu::paint::{PaintCanvas, PaintKind, DATA_KEY};
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (160, 90);
    let img = test_image(w, h);
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &img);
    let ts = TimingSettings::default();
    let t = ts.at_frame(0);
    let none = HashMap::new();
    let li = LiveInputs::default();

    // mask draw under solarize: painted pixels show the image without the effect
    let sol = EffectInstance::by_id("solarize").unwrap();
    let mut md = EffectInstance::by_id("mask_draw").unwrap();
    let mut canvas = PaintCanvas::new(PaintKind::Mask, w, h);
    for _ in 0..40 {
        for y in (0..h).step_by(2) {
            for x in (0..w / 2).step_by(2) {
                canvas.dab(x as f32, y as f32, (0.0, 0.0), false);
            }
        }
    }
    let fx = |md: &EffectInstance| vec![sol.resolve(&t, &li, &none), md.resolve(&t, &li, &none)];
    engine.render(&fx(&md), &t);
    let blank_mask = engine.read_output();
    engine.paint.insert(md.uid, canvas);
    engine.render(&fx(&md), &t);
    let live = engine.read_output();
    let at = |px: &[u8], x: u32, y: u32| px[((y * w + x) * 4) as usize..][..3].to_vec();
    // the painted mask saturates a level or two below full (8-bit blending)
    let close = |a: Vec<u8>, b: Vec<u8>| a.iter().zip(&b).all(|(x, y)| x.abs_diff(*y) <= 2);
    assert!(close(at(&live, 20, 45), at(&img, 20, 45)), "painted: original image");
    assert_eq!(at(&live, 140, 45), at(&blank_mask, 140, 45), "unpainted: effect");
    assert_ne!(at(&blank_mask, 20, 45), at(&img, 20, 45));
    // the same drawing from the preset data renders identically without a live canvas
    md.data.insert(DATA_KEY.into(), engine.paint[&md.uid].to_data_url());
    let mut export = Engine::new(&ctx, w, h);
    export.set_source_rgba(w, h, &img);
    export.render(&fx(&md), &t);
    assert!(export.read_output() == live, "stored drawing differs from the live canvas");

    // push draw: a stroke moves pixels only where it was drawn
    let pd = EffectInstance::by_id("push_draw").unwrap();
    let r = pd.resolve(&t, &li, &none);
    engine.render(std::slice::from_ref(&r), &t);
    // an empty canvas (128 = 0.502) shifts by a twelfth of a pixel, like the reference
    let out = engine.read_output();
    let mean = out.iter().zip(&img).map(|(a, b)| a.abs_diff(*b) as f64).sum::<f64>() / out.len() as f64;
    assert!(mean < 1.0, "an empty push canvas barely changes the image: {mean}");
    let mut canvas = PaintCanvas::new(PaintKind::Push, w, h);
    for _ in 0..60 {
        for x in 0..40 {
            canvas.dab(40.0 + x as f32 * 2.0, 45.0, (3.0, 0.0), false);
        }
    }
    engine.paint.insert(pd.uid, canvas);
    engine.render(std::slice::from_ref(&r), &t);
    let pushed = engine.read_output();
    assert_ne!(at(&pushed, 80, 45), at(&img, 80, 45), "stroke displaces");
    assert_eq!(at(&pushed, 80, 10), at(&out, 80, 10), "outside the stroke nothing moves");
}

#[test]
fn mouse_camera_from_effect_data() {
    use phthora_core::orbit::{Orbit, DATA_KEY};
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (96, 54);
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &test_image(w, h));
    let t = TimingSettings::default().at_frame(0);
    let (li, none) = (LiveInputs::default(), HashMap::new());
    for id in ["transform_3d", "luma_mesh"] {
        let mut e = EffectInstance::by_id(id).unwrap();
        let mut render = |e: &EffectInstance| {
            engine.render(&[e.resolve(&t, &li, &none)], &t);
            engine.read_output()
        };
        let base = render(&e);
        let mut o = Orbit::default();
        o.zoom(-8.0);
        o.rotate(30.0, 10.0, h as f32);
        e.data.insert(DATA_KEY.into(), o.to_data());
        assert!(render(&e) == base, "{id}: the camera is ignored while mouse control is off");
        e.set("mouseControls", phthora_core::Value::Bool(true));
        let moved = render(&e);
        assert!(moved != base, "{id}: the stored camera changes the view");
        e.data.insert(DATA_KEY.into(), Orbit::default().to_data());
        let back = render(&e);
        let diff = back.iter().zip(&base).filter(|(a, b)| a.abs_diff(**b) > 2).count();
        assert!(diff < back.len() / 200, "{id}: default camera looks like the fixed view ({diff} bytes differ)");
    }
}

// ---------------------------------------------------------------- Remove Background

/// The models folder is process-wide: tests that point it somewhere take turns.
static SEG_DIR: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Render Remove Background alone with the given parameters; premultiplied, top-down.
fn remove_bg(engine: &mut Engine, base: &EffectInstance, params: &[(&str, phthora_core::Value)]) -> Vec<u8> {
    // a clone keeps the uid, so the effect keeps its state between calls
    let mut e = base.clone();
    for (k, v) in params {
        e.set(k, v.clone());
    }
    let t = TimingSettings::default().at_frame(0);
    engine.render(&[e.resolve(&t, &LiveInputs::default(), &HashMap::new())], &t);
    engine.read_output()
}

/// Milliseconds per frame of the effect alone: render and wait for the GPU, no readback of the result.
fn time_frames(ctx: &GpuContext, engine: &mut Engine, base: &EffectInstance, n: u32) -> f64 {
    let t = TimingSettings::default().at_frame(0);
    let r = base.resolve(&t, &LiveInputs::default(), &HashMap::new());
    let start = std::time::Instant::now();
    for _ in 0..n {
        engine.render(std::slice::from_ref(&r), &t);
        ctx.wait();
    }
    start.elapsed().as_secs_f64() * 1000.0 / n as f64
}

/// Mask of `w`x`h` texels (top-down) that is 255 where `f(x, y)` in 0..1 coordinates holds.
fn shape_mask(w: u32, h: u32, f: impl Fn(f32, f32) -> bool) -> phthora_seg::Mask {
    let data = (0..w * h).map(|i| if f(((i % w) as f32 + 0.5) / w as f32, ((i / w) as f32 + 0.5) / h as f32) { 255 } else { 0 }).collect();
    phthora_seg::Mask { w, h, data }
}

#[test]
fn remove_background_without_model_and_with_masks() {
    let _turn = SEG_DIR.lock().unwrap_or_else(|e| e.into_inner());
    use phthora_core::Value::{Bool, Float};
    let ctx = GpuContext::headless().expect("gpu");
    let base = EffectInstance::by_id("remove_background").unwrap();
    let (w, h) = (320u32, 240u32);
    let img = test_image(w, h);
    let at = |px: &[u8], x: u32, y: u32| -> [u8; 4] { px[((y * w + x) * 4) as usize..][..4].try_into().unwrap() };

    // no model on disk: the image passes through
    let empty = std::env::temp_dir().join(format!("phthora-gpu-nomodel-{}", std::process::id()));
    phthora_seg::set_models_dir(Some(empty.clone()));
    assert!(!phthora_seg::is_installed());
    for blocking in [true, false] {
        let mut engine = Engine::new(&ctx, w, h);
        engine.set_source_rgba(w, h, &img);
        engine.blocking = blocking;
        for _ in 0..3 {
            let out = remove_bg(&mut engine, &base, &[]);
            assert!(out == img, "without a model the effect passes the image through (blocking={blocking})");
        }
        // showMask and invert do nothing either
        assert!(remove_bg(&mut engine, &base, &[("showMask", Bool(true)), ("invert", Bool(true))]) == img);
    }

    // synthetic masks: left half is the person
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &img);
    engine.background = [0.0; 4];
    engine.person_mask = Some(shape_mask(64, 36, |x, _| x < 0.5));
    let hard = [("feather", Float(0.0))];
    let out = remove_bg(&mut engine, &base, &hard);
    assert_eq!(at(&out, 40, 120), at(&img, 40, 120), "person kept");
    assert_eq!(at(&out, 280, 120)[3], 0, "background transparent");
    assert_eq!(at(&out, 140, 120)[3], 255, "kept right up to the edge");
    assert_eq!(at(&out, 180, 120)[3], 0, "removed right behind the edge");

    // feather: a gradient across the edge, centered on it
    let soft = remove_bg(&mut engine, &base, &[("feather", Float(1.0))]);
    let (a, b, c) = (at(&soft, 130, 120)[3], at(&soft, 160, 120)[3], at(&soft, 190, 120)[3]);
    assert!(a > b && b > c, "feather ramps down across the edge: {a} {b} {c}");
    assert!((60..200).contains(&b), "half coverage at the edge: {b}");
    assert!(at(&soft, 40, 120)[3] == 255 && at(&soft, 280, 120)[3] == 0, "far from the edge nothing changes");

    // expand grows the person, a negative value shrinks it
    let grown = remove_bg(&mut engine, &base, &[("feather", Float(0.0)), ("expand", Float(1.0))]);
    assert_eq!(at(&grown, 190, 120)[3], 255, "expanded past the old edge");
    assert_eq!(at(&grown, 300, 120)[3], 0, "far background stays removed");
    let shrunk = remove_bg(&mut engine, &base, &[("feather", Float(0.0)), ("expand", Float(-1.0))]);
    assert_eq!(at(&shrunk, 130, 120)[3], 0, "shrunk behind the old edge");
    assert_eq!(at(&shrunk, 40, 120)[3], 255, "core stays");

    // invert swaps person and background
    let inv = remove_bg(&mut engine, &base, &[("feather", Float(0.0)), ("invert", Bool(true))]);
    assert_eq!(at(&inv, 40, 120)[3], 0);
    assert_eq!(at(&inv, 280, 120), at(&img, 280, 120));

    // amount fades the removal: halfway the background keeps half its coverage
    let half = remove_bg(&mut engine, &base, &[("feather", Float(0.0)), ("amount", Float(0.5))]);
    assert!(at(&half, 280, 120)[3].abs_diff(128) <= 2, "{:?}", at(&half, 280, 120));
    assert_eq!(at(&half, 40, 120)[3], 255);
    assert!(remove_bg(&mut engine, &base, &[("amount", Float(0.0))]) == img, "amount 0 leaves the image alone");

    // showMask: the mask as gray, opaque; rows are not flipped (mask is a top-left quarter)
    engine.person_mask = Some(shape_mask(64, 36, |x, y| x < 0.5 && y < 0.5));
    let shown = remove_bg(&mut engine, &base, &[("feather", Float(0.0)), ("showMask", Bool(true))]);
    assert_eq!(at(&shown, 40, 30), [255, 255, 255, 255], "top left is the person");
    assert_eq!(at(&shown, 40, 200), [0, 0, 0, 255], "bottom left is not");
    assert_eq!(at(&shown, 280, 30), [0, 0, 0, 255]);
    let cut = remove_bg(&mut engine, &base, &hard);
    assert_eq!(at(&cut, 40, 30), at(&img, 40, 30));
    assert_eq!(at(&cut, 40, 200)[3], 0, "orientation of the cut-out matches the mask");

    // the background of the render shows where the person is removed
    engine.background = [0.0, 0.0, 1.0, 1.0];
    let blue = remove_bg(&mut engine, &base, &hard);
    assert_eq!(at(&blue, 280, 200), [0, 0, 255, 255]);
    phthora_seg::set_models_dir(None);
    let _ = std::fs::remove_dir_all(&empty);
}

/// Outline of the person in the CC0 test photo, in 1920x1080 pixels (see crates/seg/tests/person.rs).
const PHOTO_OUTLINE: [(f32, f32); 29] = [
    (800., 75.), (900., 62.), (1000., 95.), (1070., 180.), (1085., 260.), (1050., 330.), (1015., 415.), (1100., 485.),
    (1190., 560.), (1260., 650.), (1290., 780.), (1285., 950.), (1215., 1080.), (450., 1080.), (440., 950.), (445., 860.),
    (430., 740.), (440., 690.), (570., 665.), (650., 700.), (700., 650.), (790., 590.), (830., 540.), (850., 440.),
    (800., 400.), (760., 330.), (755., 280.), (730., 200.), (740., 150.),
];

fn in_outline(x: f32, y: f32) -> bool {
    let p = &PHOTO_OUTLINE;
    let mut c = false;
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
            c = !c;
        }
    }
    c
}

/// IoU of the output's coverage (alpha >= 128, top-down) with the outline.
fn coverage_iou(out: &[u8], w: u32, h: u32) -> f32 {
    let (mut inter, mut union) = (0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            let g = in_outline((x as f32 + 0.5) / w as f32 * 1920.0, (y as f32 + 0.5) / h as f32 * 1080.0);
            let p = out[((y * w + x) * 4 + 3) as usize] >= 128;
            inter += (g && p) as u32;
            union += (g || p) as u32;
        }
    }
    inter as f32 / union as f32
}

/// With the real network: needs PHTHORA_SEG_MODEL (the .onnx file) and PHTHORA_SEG_PHOTO (the
/// photo as PNG, 16:9); PHTHORA_SEG_OUT (a folder) additionally gets the results as PNG.
#[test]
fn remove_background_with_the_network() {
    let (Ok(model), Ok(photo)) = (std::env::var("PHTHORA_SEG_MODEL"), std::env::var("PHTHORA_SEG_PHOTO")) else {
        eprintln!("skipped: set PHTHORA_SEG_MODEL and PHTHORA_SEG_PHOTO");
        return;
    };
    let _turn = SEG_DIR.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("phthora-gpu-model-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    phthora_seg::set_models_dir(Some(dir.clone()));
    std::fs::copy(&model, phthora_seg::model_path()).unwrap();
    let base = EffectInstance::by_id("remove_background").unwrap();
    let (w, h, px) = phthora_gpu::assets::decode_png(&std::fs::read(photo).unwrap()).expect("png");
    let ctx = GpuContext::headless().expect("gpu");
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &px);
    engine.background = [0.0; 4];
    let save = |name: &str, out: &[u8]| {
        if let Ok(d) = std::env::var("PHTHORA_SEG_OUT") {
            let mut px = out.to_vec();
            phthora_gpu::context::unpremultiply(&mut px);
            let f = std::fs::File::create(std::path::Path::new(&d).join(name)).unwrap();
            let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
            enc.set_color(png::ColorType::Rgba);
            enc.write_header().unwrap().write_image_data(&px).unwrap();
        }
    };

    // export path: waits for every frame, same input gives the same mask
    assert!(engine.blocking);
    let first = remove_bg(&mut engine, &base, &[]);
    let iou = coverage_iou(&first, w, h);
    eprintln!("export: IoU {iou:.3}");
    assert!(iou > 0.85, "export IoU {iou}");
    assert!(remove_bg(&mut engine, &base, &[]) == first, "export is deterministic");
    assert_eq!(phthora_seg::status(), phthora_seg::Status::Ready);
    save("export.png", &first);
    save("mask.png", &remove_bg(&mut engine, &base, &[("showMask", phthora_core::Value::Bool(true))]));
    eprintln!("{w}x{h} effect, blocking: {:.1} ms per frame", time_frames(&ctx, &mut engine, &base, 20));

    // preview path: the first frames pass through, then the worker's mask shows up
    let mut live = Engine::new(&ctx, w, h);
    live.set_source_rgba(w, h, &px);
    live.background = [0.0; 4];
    live.blocking = false;
    let start = std::time::Instant::now();
    let mut out = remove_bg(&mut live, &base, &[]);
    while out.iter().skip(3).step_by(4).all(|&a| a == 255) && start.elapsed() < std::time::Duration::from_secs(10) {
        out = remove_bg(&mut live, &base, &[]);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    let iou = coverage_iou(&out, w, h);
    eprintln!("preview: IoU {iou:.3}, mask after {:?}", start.elapsed());
    assert!(iou > 0.85, "preview IoU {iou}");
    // steady state cost of a preview frame (readback copy and mask passes only)
    eprintln!("{w}x{h} effect, preview: {:.1} ms per frame", time_frames(&ctx, &mut live, &base, 20));
    save("preview.png", &out);
    phthora_seg::set_models_dir(None);
    let _ = std::fs::remove_dir_all(&dir);
}
