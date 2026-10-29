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
