//! Extra textures and uniforms for single-shader effects.

use super::{RenderCtx, ShaderHook};
use crate::assets::{self, VideoTexture};
use crate::context::{GpuContext, SamplerKind, Target};
use crate::shader::{TexInput, UVal};
use crate::text::{Align, TextCanvas};
use phthora_core::{EffectDesc, OutputMode, ResolvedEffect};
use std::path::PathBuf;
use std::rc::Rc;

pub mod blue_noise;

pub fn create_hook(_ctx: &GpuContext, desc: &EffectDesc) -> Option<Box<dyn ShaderHook>> {
    Some(match desc.id.as_str() {
        "eight_bit" => Box::new(Cga::default()),
        "instacolor" => Box::new(Lut::default()),
        "hard_glitch" => Box::new(Glitch::default()),
        "mask" => Box::new(ImageTex { name: "tMask", ..Default::default() }),
        "media" => Box::new(Overlay::default()),
        "vhs" => Box::new(Vhs::default()),
        "super8" => Box::new(Super8::default()),
        "ascii" => Box::new(Ascii::default()),
        "dither" => Box::new(Dither::default()),
        "caption" => Box::new(Caption::default()),
        "audio_visualizer" => Box::new(Visualizer::default()),
        "push_draw" | "mask_draw" => Box::new(PaintTex { kind: crate::paint::PaintKind::of(&desc.id)?, uploaded: None, tex: None, seed: 0.0 }),
        _ => return None,
    })
}

/// Derive an independent 0..1 value from an instance seed.
pub fn sub_seed(seed: f64, k: u32) -> f64 {
    let v = ((seed * 12.9898 + k as f64 * 78.233).sin() * 43758.5453).fract();
    v.abs()
}

fn tex(t: &Rc<Target>, sampler: SamplerKind) -> TexInput<'_> {
    TexInput { view: &t.view, sampler }
}

/// Time used to pick a frame of an overlay clip.
fn clip_time(rc: &RenderCtx, e: &ResolvedEffect, clip_seed: f64, duration: f64, image_seed_as_time: bool) -> f64 {
    if rc.ft.mode == OutputMode::Image {
        if image_seed_as_time {
            return e.f("seed") as f64 * duration;
        }
        return 0.0;
    }
    rc.ft.export_time + clip_seed * 100.0
}

// ---------------------------------------------------------------- 8-bit

#[derive(Default)]
struct Cga {
    cols: [[f32; 3]; 3],
}

impl ShaderHook for Cga {
    fn prepare(&mut self, _rc: &mut RenderCtx, e: &ResolvedEffect) {
        let c = match e.s("style") {
            "1983" => ["#ff5555", "#59ff11", "#ffff00"],
            "1984" => ["#55ffff", "#ff5650", "#ffff00"],
            _ => ["#55ffff", "#ff55ff", "#ffffff"],
        };
        self.cols = c.map(phthora_core::params::parse_hex);
    }
    fn uniform(&self, key: &str) -> Option<UVal> {
        // the reference assigns the first color to `colLight` after any param change
        match key {
            "colLight" => Some(UVal::V3(self.cols[0])),
            "colDark" => Some(UVal::V3(self.cols[1])),
            "colWhite" => Some(UVal::V3(self.cols[2])),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------- InstaColor

#[derive(Default)]
struct Lut {
    lut: Option<Rc<Target>>,
    name: String,
}

impl ShaderHook for Lut {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        let style = e.s("style");
        if self.lut.is_none() || self.name != style {
            self.lut = rc.assets.lut(rc.ctx, style);
            self.name = style.to_string();
        }
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        (name == "tLUT").then(|| self.lut.as_ref().map(|t| tex(t, SamplerKind::NearestClamp))).flatten()
    }
}

// ---------------------------------------------------------------- Hard Glitch

#[derive(Default)]
struct Glitch {
    disp: Option<Target>,
}

impl ShaderHook for Glitch {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        if self.disp.is_some() {
            return;
        }
        const N: usize = 64;
        // xorshift seeded from the instance seed
        let mut s = (e.seed * 4294967295.0) as u64 | 1;
        let mut rnd = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut data = vec![0f32; N * N * 4];
        // same fill pattern as the reference (stride 3 over a 4-channel buffer)
        for i in 0..N * N {
            let v = rnd() as f32;
            for k in 0..4 {
                data[i * 3 + k] = v;
            }
        }
        let halfs: Vec<u16> = data.iter().map(|&f| half::f16::from_f32(f).to_bits()).collect();
        let t = Target::with_format(rc.ctx, N as u32, N as u32, wgpu::TextureFormat::Rgba16Float, "glitch");
        rc.ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &t.texture, mip_level: 0, origin: Default::default(), aspect: Default::default() },
            bytemuck::cast_slice(&halfs),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(N as u32 * 8), rows_per_image: Some(N as u32) },
            wgpu::Extent3d { width: N as u32, height: N as u32, depth_or_array_layers: 1 },
        );
        self.disp = Some(t);
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        match (name, &self.disp) {
            ("tDisp", Some(t)) => Some(TexInput { view: &t.view, sampler: SamplerKind::NearestClamp }),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------- Mask (image)

#[derive(Default)]
struct ImageTex {
    name: &'static str,
    path: String,
    img: Option<Rc<Target>>,
    black: Option<Rc<Target>>,
}

impl ShaderHook for ImageTex {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        let p = e.s("path");
        if p != self.path || (self.img.is_none() && !p.is_empty()) {
            self.path = p.to_string();
            self.img = if p.is_empty() { None } else { rc.assets.image(rc.ctx, &PathBuf::from(p)).map(|(t, _)| t) };
        }
        if self.black.is_none() {
            self.black = Some(rc.assets.solid(rc.ctx, [0, 0, 0, 255]));
        }
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        if name != self.name {
            return None;
        }
        self.img.as_ref().or(self.black.as_ref()).map(|t| tex(t, SamplerKind::LinearClamp))
    }
}

// ---------------------------------------------------------------- Media overlay

#[derive(Default)]
struct Overlay {
    path: String,
    image: Option<Rc<Target>>,
    video: Option<VideoTexture>,
    aspect: f32,
    black: Option<Rc<Target>>,
}

impl ShaderHook for Overlay {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        let p = e.s("path");
        if p != self.path {
            self.path = p.to_string();
            self.image = None;
            self.video = None;
            self.aspect = 1.0;
            if !p.is_empty() {
                let path = PathBuf::from(p);
                if assets::is_image_path(&path) {
                    if let Some((t, a)) = rc.assets.image(rc.ctx, &path) {
                        self.image = Some(t);
                        self.aspect = a;
                    }
                } else {
                    let v = VideoTexture::open(rc.ctx, rc.assets, &path, [0, 0, 0, 0]);
                    let (w, h) = v.size();
                    self.aspect = w as f32 / h.max(1) as f32;
                    self.video = Some(v);
                }
            }
        }
        if let Some(v) = self.video.as_mut() {
            let t = if rc.ft.mode == OutputMode::Image { 0.0 } else { rc.ft.export_time };
            v.update(rc.ctx, t);
        }
        if self.black.is_none() {
            self.black = Some(rc.assets.solid(rc.ctx, [0, 0, 0, 255]));
        }
    }
    fn uniform(&self, key: &str) -> Option<UVal> {
        (key == "sourceAspect").then_some(UVal::F(self.aspect))
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        if name != "tOverlay" {
            return None;
        }
        let t = self.image.as_ref().or(self.video.as_ref().map(|v| &v.target)).or(self.black.as_ref())?;
        Some(tex(t, SamplerKind::LinearClamp))
    }
}

// ---------------------------------------------------------------- VHS

#[derive(Default)]
struct Vhs {
    stat: Option<VideoTexture>,
    time_tex: Option<Rc<Target>>,
    /// (canvas size, text1, text2, arrow) last drawn
    drawn: Option<((u32, u32), String, String, bool)>,
}

const VHS_CANVAS_W: u32 = 800;
const VHS_MARGIN: f32 = 28.0;

fn vhs_default_strings() -> (String, String) {
    use chrono::{Datelike, Timelike};
    let now = chrono::Local::now();
    let h24 = now.hour();
    let ampm = if h24 >= 12 { "PM" } else { "AM" };
    let h = match h24 % 12 {
        0 => 12,
        x => x,
    };
    const MONTHS: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
    (format!("{ampm} {h}:{:02}", now.minute()), format!("{} {}", MONTHS[now.month0() as usize], now.day()))
}

impl ShaderHook for Vhs {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        if self.stat.is_none() {
            let p = rc.assets.video_path("vhs-static.mp4").unwrap_or_default();
            self.stat = Some(VideoTexture::open(rc.ctx, rc.assets, &p, [0, 0, 0, 255]));
        }
        let st = self.stat.as_mut().unwrap();
        let t = clip_time(rc, e, sub_seed(e.seed, 1), st.duration(), true);
        st.update(rc.ctx, t);

        let ar = rc.width as f32 / rc.height.max(1) as f32;
        let size = (VHS_CANVAS_W, ((VHS_CANVAS_W as f32 / ar) as u32).max(1));
        let (d1, d2) = vhs_default_strings();
        let s1 = if e.s("text1").is_empty() { d1 } else { e.s("text1").to_string() };
        let s2 = if e.s("text2").is_empty() { d2 } else { e.s("text2").to_string() };
        let arrow = ((rc.ft.loop_progress * 8.0).floor() as i64) % 2 == 0;
        let key = (size, s1, s2, arrow);
        if self.drawn.as_ref() != Some(&key) {
            let fonts = rc.assets.font("VCR");
            let mut c = TextCanvas::new(size.0, size.1);
            let h = size.1 as f32;
            let white = Some([255, 255, 255]);
            c.draw_text(&fonts, 28.0, &key.1, VHS_MARGIN, h - VHS_MARGIN * 3.0, Align::Left, white, None, true);
            c.draw_text(&fonts, 28.0, &key.2, VHS_MARGIN, h - VHS_MARGIN * 2.0, Align::Left, white, None, true);
            c.draw_text(&fonts, 28.0, "PLAY", VHS_MARGIN, VHS_MARGIN, Align::Left, white, None, true);
            if arrow {
                c.draw_text(&fonts, 28.0, "\u{25B6}", VHS_MARGIN + 80.0, VHS_MARGIN - 4.0, Align::Left, white, None, true);
            }
            let data = c.to_rgba_straight();
            match &self.time_tex {
                Some(t) if t.size() == size => assets::upload(rc.ctx, t, &data, true),
                _ => self.time_tex = Some(Rc::new(assets::texture_from_rgba(rc.ctx, size.0, size.1, &data, true, "vhs-time"))),
            }
            self.drawn = Some(key);
        }
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        match name {
            "tStatic" => self.stat.as_ref().map(|v| tex(&v.target, SamplerKind::LinearClamp)),
            "tTime" => self.time_tex.as_ref().map(|t| tex(t, SamplerKind::LinearClamp)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------- Super8

#[derive(Default)]
struct Super8 {
    dust: Option<VideoTexture>,
    burn: Option<VideoTexture>,
}

impl ShaderHook for Super8 {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        if self.dust.is_none() {
            let p = rc.assets.video_path("dust.mp4").unwrap_or_default();
            // without a video backend: white dust layer (multiplied) and black burn (added) are neutral
            self.dust = Some(VideoTexture::open(rc.ctx, rc.assets, &p, [255, 255, 255, 255]));
            let p = rc.assets.video_path("burn.mp4").unwrap_or_default();
            self.burn = Some(VideoTexture::open(rc.ctx, rc.assets, &p, [0, 0, 0, 255]));
        }
        for (k, v) in [self.dust.as_mut().unwrap(), self.burn.as_mut().unwrap()].into_iter().enumerate() {
            let t = clip_time(rc, e, sub_seed(e.seed, 2 + k as u32), v.duration(), true);
            v.update(rc.ctx, t);
        }
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        match name {
            "tDust" => self.dust.as_ref().map(|v| tex(&v.target, SamplerKind::LinearClamp)),
            "tBurn" => self.burn.as_ref().map(|v| tex(&v.target, SamplerKind::LinearClamp)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------- ASCII

#[derive(Default)]
struct Ascii {
    atlas: Option<Rc<Target>>,
    set: String,
    count: usize,
}

pub fn ascii_charset(name: &str) -> &'static str {
    match name {
        "Karpinsk" => " .°*oO@",
        "Okha" => " .:+j6bHM",
        "Luza" => " .:*o&8@#",
        "Sharya" => " ._/:-+=()<>",
        "Ukhta" => " .:^\"~cvo*SO80#",
        "Donetsk" => " .`~:;!>+=icjtJY56SXDQKHNWM",
        _ => " .:-=+*%@#",
    }
}

impl ShaderHook for Ascii {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        let set = e.s("charSet");
        let custom = (set == "Custom").then(|| e.s("customChars"));
        let key = format!("{set}|{}", custom.unwrap_or_default());
        if self.atlas.is_some() && self.set == key {
            return;
        }
        const CELL: u32 = 8;
        let fonts = rc.assets.font("Pixel");
        let c = match custom {
            Some(text) => {
                // darkest to brightest by rendered coverage, one glyph per cell
                let mut chars = TextCanvas::sort_by_coverage(&fonts, CELL as f32, text);
                if chars.is_empty() {
                    chars = ascii_charset("Barysh").to_string();
                }
                let n = chars.chars().count();
                self.count = n;
                let mut c = TextCanvas::new(CELL * n as u32, CELL);
                c.fill_rect(0.0, 0.0, (CELL * n as u32) as f32, CELL as f32, [0, 0, 0, 255]);
                for (i, ch) in chars.chars().enumerate() {
                    c.draw_text(&fonts, CELL as f32, &ch.to_string(), (i as u32 * CELL) as f32, 0.0, Align::Left, Some([255, 255, 255]), None, true);
                }
                c
            }
            None => {
                let chars = ascii_charset(set);
                let n = chars.chars().count();
                self.count = n;
                let mut c = TextCanvas::new(CELL * n as u32, CELL);
                c.fill_rect(0.0, 0.0, (CELL * n as u32) as f32, CELL as f32, [0, 0, 0, 255]);
                c.draw_text(&fonts, CELL as f32, chars, 0.0, 0.0, Align::Left, Some([255, 255, 255]), None, true);
                c
            }
        };
        let data = c.to_rgba_straight();
        self.atlas = Some(Rc::new(assets::texture_from_rgba(rc.ctx, c.width(), c.height(), &data, true, "ascii")));
        self.set = key;
    }
    fn uniform(&self, key: &str) -> Option<UVal> {
        (key == "fontCharCount").then_some(UVal::F(self.count as f32))
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        (name == "tFont").then(|| self.atlas.as_ref().map(|t| tex(t, SamplerKind::NearestClamp))).flatten()
    }
}

// ---------------------------------------------------------------- Dither

#[derive(Default)]
struct Dither {
    noise: Option<Rc<Target>>,
    frame: f32,
}

impl ShaderHook for Dither {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        // animation clock: the frame index in video mode, offset by the instance seed (stills keep only the seed)
        let frame = if rc.ft.mode == OutputMode::Video { rc.ft.frame } else { 0 };
        self.frame = ((frame + (e.seed * 997.0) as u64) % 65536) as f32;
        if self.noise.is_none() && e.s("mode") == "Blue Noise" {
            let n = blue_noise::SIZE as u32;
            self.noise = Some(Rc::new(assets::texture_from_rgba(rc.ctx, n, n, &blue_noise::rgba(), false, "blue noise")));
        }
    }
    fn uniform(&self, key: &str) -> Option<UVal> {
        (key == "animFrame").then_some(UVal::F(self.frame))
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        (name == "tNoise").then(|| self.noise.as_ref().map(|t| tex(t, SamplerKind::NearestClamp))).flatten()
    }
}

// ---------------------------------------------------------------- Caption

#[derive(Default)]
struct Caption {
    tex: Option<Rc<Target>>,
    key: String,
    repeat: bool,
}

const CAPTION_CANVAS: f32 = 1600.0;
const CAPTION_FONT: f32 = 400.0;
const CAPTION_OUTLINE: f32 = 40.0;

impl ShaderHook for Caption {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        self.repeat = e.b("repeat");
        let key = format!(
            "{}x{}|{}|{}|{}|{}|{}|{:?}|{}|{:?}",
            rc.width,
            rc.height,
            e.s("text"),
            e.f("size"),
            e.f("lineHeight"),
            e.s("font"),
            e.s("align"),
            e.values.get("color"),
            e.b("outline"),
            e.values.get("outlineColor")
        );
        if self.tex.is_some() && key == self.key {
            return;
        }
        let (cw, ch) = (rc.width, rc.height);
        let scl = cw as f32 / CAPTION_CANVAS;
        let size = e.f("size");
        let padding = CAPTION_OUTLINE / 2.0 * size * scl;
        let font_size = size * CAPTION_FONT * scl;
        let line_h = font_size * e.f("lineHeight");
        let align = Align::parse(e.s("align"));
        let x = match align {
            Align::Left => padding,
            Align::Center => cw as f32 / 2.0,
            Align::Right => cw as f32 - padding,
        };
        let rgb = |k: &str| match e.values.get(k) {
            Some(phthora_core::Value::Color(c)) => c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8),
            _ => [255, 255, 255],
        };
        let fonts = rc.assets.font(e.s("font"));
        let mut c = TextCanvas::new(cw, ch);
        let lines = TextCanvas::wrap(&fonts, font_size, e.s("text"), cw as f32 - padding * 2.0);
        let stroke = e.b("outline").then(|| (rgb("outlineColor"), CAPTION_OUTLINE * size * scl));
        for (i, line) in lines.iter().enumerate() {
            c.draw_text(&fonts, font_size, line, x, padding + i as f32 * line_h, align, Some(rgb("color")), stroke, true);
        }
        // keep the edge texels empty so clamped sampling does not smear text
        c.clear_rect(0, 0, cw, 1);
        c.clear_rect(0, 0, 1, ch);
        let data = c.to_rgba_straight();
        match &self.tex {
            Some(t) if t.size() == (cw, ch) => assets::upload(rc.ctx, t, &data, true),
            _ => self.tex = Some(Rc::new(assets::texture_from_rgba(rc.ctx, cw, ch, &data, true, "caption"))),
        }
        self.key = key;
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        let s = if self.repeat { SamplerKind::LinearRepeat } else { SamplerKind::LinearClamp };
        (name == "tCaption").then(|| self.tex.as_ref().map(|t| tex(t, s))).flatten()
    }
}

// ---------------------------------------------------------------- Audio visualizer

/// Frequency bars drawn on a canvas the size of the output, blended by the shader.
#[derive(Default)]
struct Visualizer {
    tex: Option<Rc<Target>>,
    key: Option<(Vec<u32>, Vec<u32>)>,
}

/// Index into the log spectrum for a frequency (512 bins up to 22050 Hz).
fn log_index_for_freq(freq: f32) -> f32 {
    let bins = 512.0f32;
    let linear = freq / 22050.0 * (bins - 1.0);
    let log_scale = bins.ln() / bins;
    linear.max(1.0).ln() / log_scale
}

fn color_u8(e: &ResolvedEffect, k: &str) -> tiny_skia::Color {
    match e.values.get(k) {
        Some(phthora_core::Value::Color(c)) => tiny_skia::Color::from_rgba(c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0), 1.0).unwrap(),
        _ => tiny_skia::Color::WHITE,
    }
}

/// Bar heights 0..1 as the reference computes them: mean of the bins in each bar's
/// slice of 20 Hz .. 10 kHz, shaped by the edge envelope.
fn visualizer_bars(spectrum: &[f32], num_bars: usize, mirror_radial: bool, edge: f32) -> Vec<f32> {
    let min_bin = log_index_for_freq(20.0).floor();
    let max_bin = log_index_for_freq(10000.0).ceil();
    let per_bar = (max_bin - min_bin) / num_bars as f32;
    (0..num_bars)
        .map(|bar| {
            let mut idx = bar as f32;
            if mirror_radial {
                // each half of the circle shows the full range
                idx = if (bar as f32) < num_bars as f32 / 2.0 { bar as f32 * 2.0 } else { (num_bars - bar - 1) as f32 * 2.0 };
            }
            let a = (min_bin + idx * per_bar).floor() as usize;
            let b = (min_bin + (idx + 1.0) * per_bar).floor() as usize;
            let vals: Vec<f32> = (a..b).map(|i| spectrum.get(i).copied().unwrap_or(0.0)).collect();
            let mut v = if vals.is_empty() { 0.0 } else { vals.iter().sum::<f32>() / vals.len() as f32 };
            if edge > 0.0 {
                let x = bar as f32 / (num_bars as f32 - 1.0);
                v *= (4.0 * x * (1.0 - x)).max(0.0).powf(edge);
            }
            v
        })
        .collect()
}

impl ShaderHook for Visualizer {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        use tiny_skia::{GradientStop, LinearGradient, Paint, PathBuilder, Point, RadialGradient, SpreadMode, Transform};
        let (w, h) = (rc.width, rc.height);
        let num = (e.f("numBars").floor() as usize).max(1);
        let (radial, mirror) = (e.b("radial"), e.b("mirror"));
        let bars = visualizer_bars(rc.audio, num, radial && mirror, e.f("edgeDampening"));
        let cols = [color_u8(e, "color1"), color_u8(e, "color2"), color_u8(e, "color3")];
        let key = (
            bars.iter().map(|v| v.to_bits()).chain([w, h, radial as u32, mirror as u32]).collect(),
            cols.iter().flat_map(|c| [c.red(), c.green(), c.blue()]).map(f32::to_bits).collect(),
        );
        if self.tex.is_some() && self.key.as_ref() == Some(&key) {
            return;
        }
        let Some(mut pm) = tiny_skia::Pixmap::new(w.max(1), h.max(1)) else { return };
        let (wf, hf) = (w as f32, h as f32);
        let stop = |p: f32, c: tiny_skia::Color| GradientStop::new(p, c);
        let mut paint = Paint { anti_alias: true, ..Default::default() };
        if radial {
            let (cx, cy) = (wf / 2.0, hf / 2.0);
            let max_r = wf.min(hf) / 2.0;
            let inner = max_r * 0.4;
            let step = std::f32::consts::TAU / num as f32;
            if let Some(sh) = RadialGradient::new(Point::from_xy(cx, cy), inner, Point::from_xy(cx, cy), max_r, vec![stop(0.0, cols[0]), stop(0.4, cols[1]), stop(0.8, cols[2])], SpreadMode::Pad, Transform::identity()) {
                paint.shader = sh;
            }
            let half = (step * 0.9 / 2.0).tan() * inner;
            for (bar, v) in bars.iter().enumerate() {
                let ang = bar as f32 * step - std::f32::consts::FRAC_PI_2;
                let (c, s) = (ang.cos(), ang.sin());
                let end = inner + v * (max_r - inner);
                let pt = |r: f32, side: f32| (cx + c * r - s * half * side, cy + s * r + c * half * side);
                let mut pb = PathBuilder::new();
                let p0 = pt(inner, 1.0);
                pb.move_to(p0.0, p0.1);
                for (r, side) in [(end, 1.0), (end, -1.0), (inner, -1.0)] {
                    let q = pt(r, side);
                    pb.line_to(q.0, q.1);
                }
                pb.close();
                if let Some(path) = pb.finish() {
                    pm.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
                }
            }
        } else {
            let grad = if mirror {
                LinearGradient::new(Point::from_xy(0.0, 0.0), Point::from_xy(0.0, hf), vec![stop(0.1, cols[2]), stop(0.3, cols[1]), stop(0.5, cols[0]), stop(0.7, cols[1]), stop(0.9, cols[2])], SpreadMode::Pad, Transform::identity())
            } else {
                LinearGradient::new(Point::from_xy(0.0, hf), Point::from_xy(0.0, 0.0), vec![stop(0.0, cols[0]), stop(0.4, cols[1]), stop(0.8, cols[2])], SpreadMode::Pad, Transform::identity())
            };
            if let Some(sh) = grad {
                paint.shader = sh;
            }
            let bw = wf / num as f32;
            for (bar, v) in bars.iter().enumerate() {
                let bh = v * hf;
                let y = if mirror { (hf - bh) / 2.0 } else { hf - bh };
                if let Some(r) = tiny_skia::Rect::from_xywh(bar as f32 * bw, y, bw * 0.85, bh) {
                    pm.fill_rect(r, &paint, Transform::identity(), None);
                }
            }
        }
        let data: Vec<u8> = pm.pixels().iter().flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        }).collect();
        match &self.tex {
            Some(t) if t.size() == (w, h) => assets::upload(rc.ctx, t, &data, true),
            _ => self.tex = Some(Rc::new(assets::texture_from_rgba(rc.ctx, w, h, &data, true, "visualizer"))),
        }
        self.key = Some(key);
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        (name == "barsTexture").then(|| self.tex.as_ref().map(|t| tex(t, SamplerKind::LinearClamp))).flatten()
    }
}

// ---------------------------------------------------------------- Drawing effects

/// The drawing canvas as `tMask`: the editor's live canvas, or the one stored in the effect.
struct PaintTex {
    kind: crate::paint::PaintKind,
    /// what the texture shows: live canvas version, or a hash of the stored data URL
    uploaded: Option<(bool, u64, (u32, u32))>,
    tex: Option<Rc<Target>>,
    seed: f32,
}

impl ShaderHook for PaintTex {
    fn prepare(&mut self, rc: &mut RenderCtx, e: &ResolvedEffect) {
        use crate::paint::{PaintCanvas, DATA_KEY};
        use std::hash::{Hash, Hasher};
        self.seed = (sub_seed(e.seed, 5) * 100.0) as f32;
        let size = (rc.width, rc.height);
        let (key, rgba) = match rc.paint.get(&e.uid) {
            Some(c) => {
                let key = (true, c.version, c.size());
                (key, (self.uploaded != Some(key)).then(|| (c.size(), c.rgba())))
            }
            None => {
                let url = e.data.get(DATA_KEY).map(String::as_str).unwrap_or("");
                let mut h = std::collections::hash_map::DefaultHasher::new();
                url.hash(&mut h);
                let key = (false, h.finish(), size);
                (key, (self.uploaded != Some(key)).then(|| {
                    let mut c = PaintCanvas::new(self.kind, size.0, size.1);
                    c.load_data_url(url);
                    (size, c.rgba())
                }))
            }
        };
        if let Some(((w, h), data)) = rgba {
            match &self.tex {
                Some(t) if t.size() == (w, h) => assets::upload(rc.ctx, t, &data, true),
                _ => self.tex = Some(Rc::new(assets::texture_from_rgba(rc.ctx, w, h, &data, true, "paint"))),
            }
            self.uploaded = Some(key);
        }
    }
    fn uniform(&self, key: &str) -> Option<UVal> {
        // push: its own noise seed for "animate", 0..100
        (key == "seed").then_some(UVal::F(self.seed))
    }
    fn texture(&self, name: &str) -> Option<TexInput<'_>> {
        (name == "tMask").then(|| self.tex.as_ref().map(|t| tex(t, SamplerKind::LinearClamp))).flatten()
    }
}
