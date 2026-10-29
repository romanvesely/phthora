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

fn solid(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    [rgb[0], rgb[1], rgb[2], 255].repeat((w * h) as usize)
}

fn text(v: &str) -> phthora_core::Value {
    phthora_core::Value::Text(v.into())
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
