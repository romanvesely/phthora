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

// ---------------------------------------------------------------- effect modes

use phthora_core::Value;

fn fx(id: &str, params: &[(&str, Value)]) -> EffectInstance {
    let mut e = EffectInstance::by_id(id).unwrap();
    e.seed = 0.25;
    for (k, v) in params {
        e.set(k, v.clone());
    }
    e
}

fn text(s: &str) -> Value {
    Value::Text(s.to_string())
}

fn render_chain(engine: &mut Engine, chain: &[&EffectInstance], frame: u64) -> Vec<u8> {
    let t = TimingSettings::default().at_frame(frame);
    let r: Vec<_> = chain.iter().map(|e| e.resolve(&t, &LiveInputs::default(), &HashMap::new())).collect();
    engine.render(&r, &t);
    engine.read_output()
}

fn flat_engine(ctx: &GpuContext, w: u32, h: u32, gray: u8) -> Engine {
    let mut engine = Engine::new(ctx, w, h);
    engine.set_source_rgba(w, h, &[gray, gray, gray, 255].repeat((w * h) as usize));
    engine
}

fn diff_count(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

/// Writes a PNG when PHTHORA_MODES_DIR is set (visual check).
fn dump_png(name: &str, w: u32, h: u32, px: &[u8]) {
    let Ok(dir) = std::env::var("PHTHORA_MODES_DIR") else { return };
    std::fs::create_dir_all(&dir).unwrap();
    let f = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().unwrap().write_image_data(px).unwrap();
}

/// Photo-like input for the mode dumps: PHTHORA_MODES_INPUT (a PNG) or the test image.
fn mode_input() -> (u32, u32, Vec<u8>) {
    match std::env::var("PHTHORA_MODES_INPUT") {
        Ok(p) => phthora_gpu::assets::decode_png(&std::fs::read(p).unwrap()).expect("png"),
        Err(_) => (320, 240, test_image(320, 240)),
    }
}

const DITHER_MODES: [&str; 6] = ["Bayer 4x4", "Bayer 2x2", "Bayer 8x8", "Blue Noise", "Random", "Cluster Dot"];

#[test]
fn dither_modes_differ_and_repeat() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h, img) = mode_input();
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &img);
    let mut outs: Vec<Vec<u8>> = vec![];
    for (i, mode) in DITHER_MODES.iter().enumerate() {
        let e = fx("dither", &[("mode", text(mode))]);
        let a = render_chain(&mut engine, &[&e], 3);
        let b = render_chain(&mut engine, &[&e], 3);
        assert!(a == b, "{mode}: not deterministic");
        // black and white only
        assert!(a.as_chunks::<4>().0.iter().all(|p| (p[0] == 0 || p[0] == 255) && p[0] == p[1] && p[1] == p[2]), "{mode}: not binary");
        dump_png(&format!("dither_{i}_{}", mode.replace(' ', "_")), w, h, &a);
        outs.push(a);
    }
    for i in 0..outs.len() {
        for j in i + 1..outs.len() {
            let d = diff_count(&outs[i], &outs[j]);
            assert!(d > outs[i].len() / 200, "{} and {} look the same ({d} bytes differ)", DITHER_MODES[i], DITHER_MODES[j]);
        }
    }
    // a fresh engine renders the same frame identically (export = preview)
    let mut other = Engine::new(&ctx, w, h);
    other.set_source_rgba(w, h, &img);
    let e = fx("dither", &[("mode", text("Blue Noise"))]);
    assert!(render_chain(&mut other, &[&e], 3) == outs[3]);
}

#[test]
fn dither_animation_follows_the_frame() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (128, 96);
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &test_image(w, h));
    for mode in DITHER_MODES {
        let still = fx("dither", &[("mode", text(mode))]);
        assert!(render_chain(&mut engine, &[&still], 0) == render_chain(&mut engine, &[&still], 7), "{mode}: moves without animate");
        let anim = fx("dither", &[("mode", text(mode)), ("animate", Value::Bool(true))]);
        let f0 = render_chain(&mut engine, &[&anim], 0);
        let f1 = render_chain(&mut engine, &[&anim], 1);
        assert!(f0 != f1, "{mode}: animate keeps the pattern");
        assert!(f0 == render_chain(&mut engine, &[&anim], 0), "{mode}: animation is not deterministic");
        assert!(f1 == render_chain(&mut engine, &[&anim], 1), "{mode}: animation is not deterministic");
    }
}

#[test]
fn dither_modes_hit_their_gray_levels() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (64, 64);
    // flat 25% gray: the fraction of lit pixels follows the gray level in every matrix
    let mut engine = flat_engine(&ctx, w, h, 64);
    let lit = |px: &[u8]| px.as_chunks::<4>().0.iter().filter(|p| p[0] == 255).count() as f32 / (w * h) as f32;
    for mode in ["Bayer 4x4", "Bayer 2x2", "Bayer 8x8", "Blue Noise"] {
        let e = fx("dither", &[("mode", text(mode)), ("scale", Value::Float(1.0))]);
        let got = lit(&render_chain(&mut engine, &[&e], 0));
        assert!((got - 0.25).abs() < 0.01, "{mode}: {got} lit, expected 0.25");
    }
}

#[test]
fn blue_noise_map_is_a_permutation_and_evenly_spread() {
    use phthora_gpu::effects::hooks::blue_noise::{ranks, rgba, SIZE};
    let r = ranks();
    assert_eq!(r.len(), SIZE * SIZE);
    let mut seen = vec![false; SIZE * SIZE];
    for &v in r {
        assert!(!std::mem::replace(&mut seen[v as usize], true), "rank {v} twice");
    }
    assert!(seen.iter().all(|s| *s));
    // texels rebuild the rank exactly
    for (i, px) in rgba().as_chunks::<4>().0.iter().enumerate() {
        assert_eq!(px[0] as u16 * 16 + px[1] as u16 / 16, r[i]);
    }
    // stable across calls
    assert!(ranks() == r);
    // evenly spread: the lower half of the ranks puts nearly the same count into every 4x4 block
    let half = (SIZE * SIZE / 2) as u16;
    let mut counts = vec![];
    for by in 0..SIZE / 4 {
        for bx in 0..SIZE / 4 {
            let n = (0..16).filter(|k| r[(by * 4 + k / 4) * SIZE + bx * 4 + k % 4] < half).count();
            counts.push(n as f64);
        }
    }
    let mean = counts.iter().sum::<f64>() / counts.len() as f64;
    let sd = (counts.iter().map(|c| (c - mean).powi(2)).sum::<f64>() / counts.len() as f64).sqrt();
    println!("blue noise: 4x4 block counts mean {mean:.2} sd {sd:.2}");
    assert!((mean - 8.0).abs() < 0.01);
    // white noise would give sd 2.0
    assert!(sd < 1.3, "blocks vary too much: {sd}");
    // not a Bayer matrix: no period of 8 and no ordered rows
    let same = (0..SIZE * SIZE).filter(|&i| r[i] == r[(i / SIZE * SIZE) + (i % SIZE + 8) % SIZE]).count();
    assert!(same < 50, "{same} texels repeat with period 8");
    let sorted_rows = (0..SIZE).filter(|&y| r[y * SIZE..(y + 1) * SIZE].windows(2).all(|p| p[0] < p[1])).count();
    assert_eq!(sorted_rows, 0);
}

/// Mean step between neighbouring pixels of the added noise (out - orig), red channel.
fn roughness(px: &[u8], orig: &[u8], w: u32) -> f64 {
    let mut sum = 0f64;
    let mut n = 0u32;
    for i in 0..px.len() / 4 - 1 {
        if (i as u32 + 1).is_multiple_of(w) {
            continue;
        }
        let a = px[i * 4] as f64 - orig[i * 4] as f64;
        let b = px[i * 4 + 4] as f64 - orig[i * 4 + 4] as f64;
        sum += (a - b).abs();
        n += 1;
    }
    sum / n as f64
}

#[test]
fn grain_types() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h, img) = mode_input();
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &img);
    let base = render_chain(&mut engine, &[], 0);
    let params = [("amount", Value::Float(1.0)), ("blendMode", text("additive"))];
    let digital = fx("grain", &params);
    let mut film_p = params.to_vec();
    film_p.push(("type", text("Film")));
    let film = fx("grain", &film_p);
    let d = render_chain(&mut engine, &[&digital], 5);
    let f = render_chain(&mut engine, &[&film], 5);
    dump_png("grain_digital", w, h, &d);
    dump_png("grain_film", w, h, &f);
    assert!(d != f);
    assert!(f == render_chain(&mut engine, &[&film], 5), "film is not deterministic");
    let (rd, rf) = (roughness(&d, &base, w), roughness(&f, &base, w));
    println!("grain roughness: digital {rd:.1}, film {rf:.1}");
    assert!(rf < rd * 0.8, "film grain should be softer than digital ({rf} vs {rd})");
    // film moves with time and with speed; frozen at speed 0
    assert!(f != render_chain(&mut engine, &[&film], 6));
    let mut frozen_p = film_p.clone();
    frozen_p.push(("speed", Value::Float(0.0)));
    let frozen = fx("grain", &frozen_p);
    assert!(render_chain(&mut engine, &[&frozen], 5) == render_chain(&mut engine, &[&frozen], 6));
    let mut fast_p = film_p.clone();
    fast_p.push(("speed", Value::Float(3.0)));
    assert!(f != render_chain(&mut engine, &[&fx("grain", &fast_p)], 5));
    // scale changes the grain size in both types
    for (name, base_p) in [("digital", params.to_vec()), ("film", film_p.clone())] {
        let mut big = base_p.clone();
        big.push(("scale", Value::Float(4.0)));
        let b = render_chain(&mut engine, &[&fx("grain", &big)], 5);
        let n = render_chain(&mut engine, &[&fx("grain", &base_p)], 5);
        assert!(b != n, "{name}: scale has no effect");
        assert!(roughness(&b, &base, w) < roughness(&n, &base, w), "{name}: larger scale should be coarser");
    }
}

#[test]
fn ascii_custom_characters() {
    use phthora_gpu::text::TextCanvas;
    let ctx = GpuContext::headless().expect("gpu");
    // sorted by rendered coverage, duplicates and control characters dropped
    let fonts = phthora_gpu::assets::Assets::new().font("Pixel");
    let sorted = TextCanvas::sort_by_coverage(&fonts, 8.0, "@#.\n @");
    println!("sorted: {sorted:?}");
    assert_eq!(sorted.chars().count(), 4);
    assert_eq!(sorted.chars().next(), Some(' '));
    assert_eq!(sorted.chars().nth(1), Some('.'));
    assert!(sorted.ends_with('@') || sorted.ends_with('#'));
    assert_eq!(TextCanvas::sort_by_coverage(&fonts, 8.0, "# .@"), TextCanvas::sort_by_coverage(&fonts, 8.0, "@. #"));

    let (w, h, img) = mode_input();
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &img);
    let default = fx("ascii", &[]);
    let mut custom = fx("ascii", &[("charSet", text("Custom")), ("customChars", text("@. #"))]);
    let d = render_chain(&mut engine, &[&default], 0);
    let a = render_chain(&mut engine, &[&custom], 0);
    dump_png("ascii_default", w, h, &d);
    dump_png("ascii_custom", w, h, &a);
    assert!(a != d);
    assert!(a == render_chain(&mut engine, &[&custom], 0));
    // the order of the typed characters does not matter
    custom.set("customChars", text("# .@"));
    assert!(render_chain(&mut engine, &[&custom], 0) == a);
    // changing the text rebuilds the atlas
    custom.set("customChars", text("MW:."));
    let b = render_chain(&mut engine, &[&custom], 0);
    dump_png("ascii_custom2", w, h, &b);
    assert!(b != a);
    // empty text falls back to a built-in set instead of failing
    custom.set("customChars", text(""));
    let e = render_chain(&mut engine, &[&custom], 0);
    assert!(e != a && e.len() == a.len());
}

#[test]
fn chroma_key_mask_and_source() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (96, 64);
    // green panel on gray
    let mut img = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let green = (16..48).contains(&x) && (16..48).contains(&y);
            img.extend_from_slice(if green { &[0x46, 0xff, 0x03, 255] } else { &[120, 90, 160, 255] });
        }
    }
    let mut engine = Engine::new(&ctx, w, h);
    engine.set_source_rgba(w, h, &img);
    let at = |px: &[u8], x: u32, y: u32| px[((y * w + x) * 4) as usize..][..4].to_vec();

    // showMask: white where the key colour is, black elsewhere
    let key = fx("chroma_key", &[]);
    let shown = fx("chroma_key", &[("showMask", Value::Bool(true))]);
    let plain = render_chain(&mut engine, &[&key], 0);
    let mask = render_chain(&mut engine, &[&shown], 0);
    assert_eq!(at(&mask, 30, 30), vec![255, 255, 255, 255]);
    assert_eq!(at(&mask, 80, 30), vec![0, 0, 0, 255]);
    assert!(mask.as_chunks::<4>().0.iter().all(|p| (p[0] == 0 || p[0] == 255) && p[0] == p[1] && p[1] == p[2]));
    assert!(mask == render_chain(&mut engine, &[&shown], 0));
    assert!(mask != plain);
    dump_png("chroma_mask", w, h, &mask);

    // mask from the source: a colour effect in front hides the key in the effect input
    let pre = fx("hue_cycle", &[]);
    let from_input = fx("chroma_key", &[("showMask", Value::Bool(true))]);
    let from_source = fx("chroma_key", &[("showMask", Value::Bool(true)), ("maskFrom", text("Source"))]);
    let shifted = render_chain(&mut engine, &[&pre], 0);
    assert!(at(&shifted, 30, 30) != vec![0x46, 0xff, 0x03, 255], "the colour effect must move the key colour");
    let m_in = render_chain(&mut engine, &[&pre, &from_input], 0);
    let m_src = render_chain(&mut engine, &[&pre, &from_source], 0);
    assert_eq!(at(&m_src, 30, 30), vec![255, 255, 255, 255], "source mask finds the panel");
    assert_eq!(at(&m_in, 30, 30), vec![0, 0, 0, 255], "input mask no longer finds it");
    assert_eq!(at(&m_src, 80, 30), vec![0, 0, 0, 255]);
    dump_png("chroma_mask_source", w, h, &m_src);
    // as a keyer: the source mask cuts the panel out of the effect result
    let cut = fx("chroma_key", &[("maskFrom", text("Source"))]);
    let keyed = render_chain(&mut engine, &[&pre, &cut], 0);
    let uncut = render_chain(&mut engine, &[&pre, &key], 0);
    assert!(keyed != uncut);
    assert_eq!(keyed, render_chain(&mut engine, &[&pre, &cut], 0));
}

// ---------------------------------------------------------------- luma mesh modes, slit scan

fn solid(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    [rgb[0], rgb[1], rgb[2], 255].repeat((w * h) as usize)
}

fn float(v: f32) -> phthora_core::Value {
    phthora_core::Value::Float(v)
}

/// Renders effects frame by frame on one engine and returns the top-down output.
struct Rig {
    engine: Engine,
    ts: TimingSettings,
    frame: u64,
}

impl Rig {
    fn new(ctx: &GpuContext, w: u32, h: u32) -> Self {
        Self { engine: Engine::new(ctx, w, h), ts: TimingSettings::default(), frame: 0 }
    }

    fn render(&mut self, e: &EffectInstance) -> Vec<u8> {
        let t = self.ts.at_frame(self.frame);
        self.engine.render(&[e.resolve(&t, &LiveInputs::default(), &HashMap::new())], &t);
        self.engine.read_output()
    }

    /// Next frame of a sequence: upload the source, render, advance time.
    fn step(&mut self, e: &EffectInstance, w: u32, h: u32, src: &[u8]) -> Vec<u8> {
        self.engine.set_source_rgba(w, h, src);
        let out = self.render(e);
        self.frame += 1;
        out
    }
}

fn luma_mesh_with(params: &[(&str, phthora_core::Value)]) -> EffectInstance {
    let mut e = EffectInstance::by_id("luma_mesh").unwrap();
    e.seed = 0.37;
    for (k, v) in params {
        e.set(k, v.clone());
    }
    e
}

fn lit(px: &[u8]) -> usize {
    px.as_chunks::<4>().0.iter().filter(|p| p[0] > 20 || p[1] > 20 || p[2] > 20).count()
}

fn diff(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

#[test]
fn luma_mesh_lines_and_points() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (160, 90);
    let mut rig = Rig::new(&ctx, w, h);
    rig.engine.set_source_rgba(w, h, &test_image(w, h));
    rig.frame = 7;
    let mesh = rig.render(&luma_mesh_with(&[]));
    let lines = rig.render(&luma_mesh_with(&[("mode", text("Lines"))]));
    let points = rig.render(&luma_mesh_with(&[("mode", text("Points"))]));
    assert!(mesh != lines && mesh != points && lines != points, "the three modes look different");
    assert!(lit(&lines) > 200 && lit(&points) > 200, "lines {} points {}", lit(&lines), lit(&points));
    assert!(lit(&mesh) > 0);

    // deterministic: the same frame again, and on a fresh engine
    let mut other = Rig::new(&ctx, w, h);
    other.engine.set_source_rgba(w, h, &test_image(w, h));
    other.frame = 7;
    for (mode, first) in [("Lines", &lines), ("Points", &points)] {
        let e = luma_mesh_with(&[("mode", text(mode))]);
        assert!(&rig.render(&e) == first, "{mode}: same frame twice");
        assert!(&other.render(&e) == first, "{mode}: fresh engine");
    }

    // resolution changes the geometry; the other params change their mode
    let res = |mode: &str, r: f32, rig: &mut Rig| rig.render(&luma_mesh_with(&[("mode", text(mode)), ("resolution", float(r)), ("gridOpacity", float(1.0))]));
    for mode in ["Mesh", "Lines", "Points"] {
        let (a, b) = (res(mode, 30.0, &mut rig), res(mode, 120.0, &mut rig));
        assert!(diff(&a, &b) > 100, "{mode}: resolution changes the picture");
    }
    // the default is the original 120 segments
    let default = rig.render(&luma_mesh_with(&[("gridOpacity", float(1.0))]));
    assert!(default == res("Mesh", 120.0, &mut rig), "default resolution is 120");
    let count = |n: f32, rig: &mut Rig| rig.render(&luma_mesh_with(&[("mode", text("Lines")), ("lineCount", float(n))]));
    assert!(count(20.0, &mut rig) != count(40.0, &mut rig));
    let thick = |v: f32, rig: &mut Rig| rig.render(&luma_mesh_with(&[("mode", text("Lines")), ("lineThickness", float(v))]));
    assert!(lit(&thick(1.0, &mut rig)) < lit(&thick(6.0, &mut rig)));
    let blur = |v: f32, rig: &mut Rig| rig.render(&luma_mesh_with(&[("mode", text("Lines")), ("lineBlur", float(v))]));
    assert!(lit(&blur(0.0, &mut rig)) < lit(&blur(0.8, &mut rig)), "blur widens the lines");
    let size = |v: f32, rig: &mut Rig| rig.render(&luma_mesh_with(&[("mode", text("Points")), ("resolution", float(30.0)), ("pointSize", float(v))]));
    assert!(lit(&size(2.0, &mut rig)) < lit(&size(8.0, &mut rig)));
    let opacity = |v: f32, rig: &mut Rig| rig.render(&luma_mesh_with(&[("mode", text("Lines")), ("lineOpacity", float(v))]));
    assert!(opacity(1.0, &mut rig) != opacity(0.4, &mut rig));
    assert!(lit(&opacity(0.0, &mut rig)) == 0, "opacity 0 draws nothing");

    // blend: on a grey background the three modes differ
    rig.engine.background = [0.4, 0.4, 0.4, 1.0];
    for mode in ["Lines", "Points"] {
        let b = |blend: &str, rig: &mut Rig| {
            rig.render(&luma_mesh_with(&[("mode", text(mode)), ("blend", text(blend)), ("lineThickness", float(4.0)), ("lineBlur", float(0.3))]))
        };
        let (n, a, s) = (b("Normal", &mut rig), b("Additive", &mut rig), b("Screen", &mut rig));
        assert!(n != a && n != s && a != s, "{mode}: blend modes differ");
    }
    // the mesh ignores the blend setting
    let m = |blend: &str, rig: &mut Rig| rig.render(&luma_mesh_with(&[("blend", text(blend))]));
    assert!(m("Normal", &mut rig) == m("Screen", &mut rig));
    rig.engine.background = [0.0, 0.0, 0.0, 1.0];

    // tilt and the mouse camera move every mode
    use phthora_core::orbit::{Orbit, DATA_KEY};
    for mode in ["Mesh", "Lines", "Points"] {
        let base = rig.render(&luma_mesh_with(&[("mode", text(mode))]));
        let tilted = rig.render(&luma_mesh_with(&[("mode", text(mode)), ("tiltAmount", float(0.8)), ("tiltSpeed", float(0.7))]));
        assert!(tilted != base, "{mode}: tilt");
        let mut e = luma_mesh_with(&[("mode", text(mode)), ("mouseControls", phthora_core::Value::Bool(true))]);
        let mut o = Orbit::default();
        o.zoom(-8.0);
        o.rotate(30.0, 10.0, h as f32);
        e.data.insert(DATA_KEY.into(), o.to_data());
        assert!(rig.render(&e) != base, "{mode}: mouse camera");
        e.data.insert(DATA_KEY.into(), Orbit::default().to_data());
        let back = rig.render(&e);
        assert!(diff(&back, &base) < back.len() / 100, "{mode}: default camera is the fixed view");
    }
}

#[test]
fn luma_mesh_sizes_are_output_pixels() {
    let ctx = GpuContext::headless().expect("gpu");
    // coverage along a line through the middle adds up to the size in pixels, at any output size
    for (w, h) in [(160u32, 90u32), (320, 180)] {
        let mut rig = Rig::new(&ctx, w, h);
        rig.engine.set_source_rgba(w, h, &solid(w, h, [255, 255, 255]));
        let at = |px: &[u8], x: u32, y: u32| px[((y * w + x) * 4) as usize] as f32 / 255.0;
        // three flat lines: the middle one is horizontal through the centre
        let flat = [("mode", text("Lines")), ("lumaHeight", float(0.0)), ("lineCount", float(3.0)), ("lineThickness", float(8.0))];
        let lines = rig.render(&luma_mesh_with(&flat));
        let cover: f32 = (h / 2 - 12..h / 2 + 12).map(|y| at(&lines, w / 3, y)).sum();
        assert!((cover - 8.0).abs() < 0.75, "{w}x{h}: line thickness {cover}");
        let soft = rig.render(&luma_mesh_with(&[flat[0].clone(), flat[1].clone(), flat[2].clone(), flat[3].clone(), ("lineBlur", float(0.5))]));
        let spread = (h / 2 - 30..h / 2 + 30).filter(|&y| at(&soft, w / 3, y) > 0.02).count();
        let sharp = (h / 2 - 30..h / 2 + 30).filter(|&y| at(&lines, w / 3, y) > 0.02).count();
        assert!(spread > sharp + 8, "{w}x{h}: blur spreads the edge ({sharp} -> {spread})");
        // a point on the centre vertex (even segment count)
        let points = rig.render(&luma_mesh_with(&[("mode", text("Points")), ("lumaHeight", float(0.0)), ("resolution", float(20.0)), ("pointSize", float(5.0))]));
        let cover: f32 = (w / 2 - 6..w / 2 + 6).map(|x| at(&points, x, h / 2)).sum();
        assert!((cover - 5.0).abs() < 0.75, "{w}x{h}: point size {cover}");
    }
}

/// Frame k of a test sequence: red encodes k, so a pixel tells which frame it shows.
fn seq_frame(w: u32, h: u32, k: u32) -> Vec<u8> {
    solid(w, h, [5 + 10 * k as u8, 100, 200])
}

fn slit(params: &[(&str, phthora_core::Value)]) -> EffectInstance {
    let mut e = EffectInstance::by_id("slit_scan").unwrap();
    for (k, v) in params {
        e.set(k, v.clone());
    }
    e
}

#[test]
fn slit_scan_stripes_show_delayed_frames() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (160, 90);
    let n = 8u32;
    let red = |px: &[u8], x: u32, y: u32| px[((y * w + x) * 4) as usize] as i32;
    let stripe_x = |i: u32| (i * w / n) + w / n / 2;
    let hard = slit(&[("slices", float(n as f32)), ("amount", float(1.0))]);
    let mut rig = Rig::new(&ctx, w, h);
    let mut last = vec![];
    for k in 0..12u32 {
        last = rig.step(&hard, w, h, &seq_frame(w, h, k));
        // stripe i shows the frame i back, the oldest recorded one before the ring is full
        for i in 0..n {
            let want = 5 + 10 * k.saturating_sub(i) as i32;
            let got = red(&last, stripe_x(i), h / 2);
            assert_eq!(got, want, "frame {k}, stripe {i}");
            assert_eq!(last[((h / 2 * w + stripe_x(i)) * 4 + 1) as usize], 100, "other channels pass");
        }
    }
    // the same sequence on a fresh engine gives the same picture
    let mut again = Rig::new(&ctx, w, h);
    let mut out = vec![];
    for k in 0..12u32 {
        out = again.step(&hard, w, h, &seq_frame(w, h, k));
    }
    assert!(out == last, "deterministic");

    // amount scales the delay: half of it, rounded to whole frames
    let half = slit(&[("slices", float(n as f32)), ("amount", float(0.5))]);
    let mut rig = Rig::new(&ctx, w, h);
    for k in 0..12u32 {
        last = rig.step(&half, w, h, &seq_frame(w, h, k));
    }
    for (i, delay) in [(0u32, 0i32), (2, 1), (5, 3), (7, 4)] {
        assert_eq!(red(&last, stripe_x(i), h / 2), 5 + 10 * (11 - delay), "amount 0.5, stripe {i}");
    }
    let zero = slit(&[("slices", float(n as f32)), ("amount", float(0.0))]);
    let out = rig.step(&zero, w, h, &seq_frame(w, h, 12));
    assert!(out == seq_frame(w, h, 12), "amount 0 shows the live frame");

    // smooth: between the two neighbouring delays
    let smooth = slit(&[("slices", float(n as f32)), ("amount", float(1.0)), ("smooth", phthora_core::Value::Bool(true))]);
    let mut rig = Rig::new(&ctx, w, h);
    for k in 0..12u32 {
        last = rig.step(&smooth, w, h, &seq_frame(w, h, k));
    }
    // right at the middle the delay is 3.5 frames
    let mid = red(&last, w / 2, h / 2);
    assert!((mid - (5 + 10 * 11 - 35)).abs() <= 2, "smooth middle {mid}");
    let (a, b) = (red(&last, w / 2 - 20, h / 2), red(&last, w / 2 + 20, h / 2));
    assert!(a > mid && mid > b, "delay grows to the right: {a} {mid} {b}");
    let distinct: std::collections::BTreeSet<i32> = (0..w).map(|x| red(&last, x, h / 2)).collect();
    assert!(distinct.len() > 3 * n as usize, "gradient, not stripes: {} values", distinct.len());

    // angle: a quarter turn makes horizontal stripes, the delay growing from bottom to top
    let up = slit(&[("slices", float(n as f32)), ("amount", float(1.0)), ("angle", float(0.25))]);
    let mut rig = Rig::new(&ctx, w, h);
    for k in 0..12u32 {
        last = rig.step(&up, w, h, &seq_frame(w, h, k));
    }
    let row = |i: u32| h - 1 - (i * h / n + h / n / 2);
    for i in 0..n {
        assert_eq!(red(&last, w / 2, row(i)), 5 + 10 * (11 - i as i32), "angle 0.25, stripe {i} from the bottom");
    }
    assert_eq!(red(&last, 3, row(4)), red(&last, w - 4, row(4)), "stripes are horizontal");
}

#[test]
fn slit_scan_reset_and_image_mode() {
    let ctx = GpuContext::headless().expect("gpu");
    let (w, h) = (96, 54);
    let e = slit(&[("slices", float(6.0)), ("amount", float(1.0))]);
    let red = |px: &[u8], x: u32, y: u32| px[((y * w + x) * 4) as usize];
    let mut rig = Rig::new(&ctx, w, h);
    let mut out = vec![];
    for k in 0..10u32 {
        out = rig.step(&e, w, h, &seq_frame(w, h, k));
    }
    assert!(red(&out, 3, 10) != red(&out, w - 3, 10), "stripes differ before the reset");
    // reset empties the ring: only the new frame is left, so every stripe shows it
    rig.engine.reset_history();
    let out = rig.step(&e, w, h, &seq_frame(w, h, 10));
    assert!(out == seq_frame(w, h, 10), "after a reset all stripes show the live frame");
    let out = rig.step(&e, w, h, &seq_frame(w, h, 11));
    assert_eq!(red(&out, 3, 10), 5 + 110);
    assert_eq!(red(&out, w - 3, 10), 5 + 100, "the ring starts over after a reset");

    // an image has no time: passed through, whatever was recorded before
    let mut still = Rig::new(&ctx, w, h);
    still.ts.mode = phthora_core::OutputMode::Image;
    for k in 0..6u32 {
        let out = still.step(&e, w, h, &seq_frame(w, h, k));
        assert!(out == seq_frame(w, h, k), "image mode passes frame {k} through");
    }
    let test = test_image(w, h);
    assert!(still.step(&e, w, h, &test) == test, "image mode keeps a real picture exactly");
    // switching to video starts with an empty ring
    still.ts.mode = phthora_core::OutputMode::Video;
    let out = still.step(&e, w, h, &seq_frame(w, h, 7));
    assert!(out == seq_frame(w, h, 7));
}

#[test]
fn slit_scan_tall_frames_use_a_smaller_ring() {
    let ctx = GpuContext::headless().expect("gpu");
    // taller than 1080: the ring frames are scaled down, the live stripe stays at full size
    let (w, h) = (512, 1200);
    let e = slit(&[("slices", float(4.0)), ("amount", float(1.0))]);
    let mut rig = Rig::new(&ctx, w, h);
    let mut out = vec![];
    for k in 0..6u32 {
        out = rig.step(&e, w, h, &seq_frame(w, h, k));
    }
    let red = |x: u32, y: u32| out[((y * w + x) * 4) as usize] as i32;
    for i in 0..4u32 {
        assert_eq!(red(i * w / 4 + w / 8, h / 2), 5 + 10 * (5 - i as i32), "tall frame, stripe {i}");
    }
}
