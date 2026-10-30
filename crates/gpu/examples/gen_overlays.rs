//! Generates the overlay clips in `assets/video` (dust, burn, vhs-static).
//!
//! Everything is drawn procedurally with a fixed seed, so the output is
//! reproducible. Frames are piped to ffmpeg as raw RGB24.
//!
//!     cargo run -p phthora-gpu --example gen_overlays --release -- assets/video

use std::io::Write;
use std::process::{Command, Stdio};
use tiny_skia::{
    BlendMode, Color, FillRule, GradientStop, LineCap, Paint, PathBuilder, Pixmap, Point, RadialGradient, SpreadMode, Stroke,
    Transform,
};

/// Frames blended from the end of the clip into its start so that it loops.
const FADE: usize = 15;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    fn int(&mut self, a: u32, b: u32) -> u32 {
        a + (self.next() % (b - a + 1) as u64) as u32
    }
}

trait Layer {
    /// Next frame as RGB24.
    fn frame(&mut self) -> Vec<u8>;
}

fn solid(c: f32, a: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(Color::from_rgba(c, c, c, a.clamp(0.0, 1.0)).unwrap());
    p.anti_alias = true;
    p
}

fn to_rgb(pm: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pm.pixels().len() * 3);
    for p in pm.pixels() {
        out.extend_from_slice(&[p.red(), p.green(), p.blue()]);
    }
    out
}

// ------------------------------------------------------------------ dust

struct Hair {
    p: [(f32, f32); 4],
    vel: (f32, f32),
    age: u32,
    life: u32,
    width: f32,
}

struct Scratch {
    x: f32,
    y0: f32,
    y1: f32,
    left: u32,
    a: f32,
}

struct Dust {
    rng: Rng,
    w: u32,
    h: u32,
    hairs: Vec<Hair>,
    scratches: Vec<Scratch>,
}

impl Dust {
    fn new(w: u32, h: u32) -> Self {
        Dust { rng: Rng::new(11), w, h, hairs: Vec::new(), scratches: Vec::new() }
    }
}

impl Layer for Dust {
    fn frame(&mut self) -> Vec<u8> {
        let (w, h) = (self.w as f32, self.h as f32);
        let r = &mut self.rng;
        let mut pm = Pixmap::new(self.w, self.h).unwrap();
        // film base: white with flicker, soft vignette and a little grain are applied below
        let flicker = r.range(0.92, 1.0);
        pm.fill(Color::from_rgba(flicker, flicker, flicker, 1.0).unwrap());

        // dust specks
        for _ in 0..r.int(20, 60) {
            let (x, y) = (r.range(0.0, w), r.range(0.0, h));
            let rad = r.range(0.5, 2.0);
            let a = r.range(0.3, 0.8);
            if let Some(path) = PathBuilder::from_circle(x, y, rad) {
                pm.fill_path(&path, &solid(0.0, a), FillRule::Winding, Transform::identity(), None);
            }
        }

        // hairs
        while self.hairs.len() < 3 && r.f() < 0.12 {
            let (x, y) = (r.range(0.0, w), r.range(0.0, h));
            let len = r.range(20.0, 90.0);
            let ang = r.range(0.0, std::f32::consts::TAU);
            let (dx, dy) = (ang.cos(), ang.sin());
            let mut jit = || r.range(-len * 0.35, len * 0.35);
            let p = [
                (x, y),
                (x + dx * len * 0.33 - dy * jit(), y + dy * len * 0.33 + dx * jit()),
                (x + dx * len * 0.66 - dy * jit(), y + dy * len * 0.66 + dx * jit()),
                (x + dx * len, y + dy * len),
            ];
            let life = r.int(10, 40);
            self.hairs.push(Hair { p, vel: (r.range(-1.2, 1.2), r.range(-1.2, 1.2)), age: 0, life, width: r.range(0.5, 1.0) });
        }
        if self.hairs.is_empty() && r.f() < 0.5 {
            self.hairs.clear();
        }
        for hair in &mut self.hairs {
            let mut pb = PathBuilder::new();
            pb.move_to(hair.p[0].0, hair.p[0].1);
            pb.cubic_to(hair.p[1].0, hair.p[1].1, hair.p[2].0, hair.p[2].1, hair.p[3].0, hair.p[3].1);
            if let Some(path) = pb.finish() {
                let fade = (hair.age.min(hair.life - hair.age) as f32 / 3.0).min(1.0);
                let stroke = Stroke { width: hair.width, line_cap: LineCap::Round, ..Stroke::default() };
                pm.stroke_path(&path, &solid(0.0, 0.55 * fade), &stroke, Transform::identity(), None);
            }
            for p in &mut hair.p {
                p.0 += hair.vel.0;
                p.1 += hair.vel.1;
            }
            hair.age += 1;
        }
        self.hairs.retain(|h| h.age < h.life);

        // vertical scratches
        if r.f() < 0.05 {
            let y0 = r.range(0.0, h * 0.5);
            self.scratches.push(Scratch {
                x: r.range(0.0, w),
                y0,
                y1: y0 + r.range(h * 0.3, h),
                left: r.int(3, 8),
                a: r.range(0.25, 0.6),
            });
        }
        for s in &mut self.scratches {
            let mut pb = PathBuilder::new();
            pb.move_to(s.x, s.y0);
            pb.line_to(s.x + r.range(-0.6, 0.6), s.y1);
            if let Some(path) = pb.finish() {
                let stroke = Stroke { width: 1.0, ..Stroke::default() };
                pm.stroke_path(&path, &solid(0.0, s.a), &stroke, Transform::identity(), None);
            }
            s.left -= 1;
        }
        self.scratches.retain(|s| s.left > 0);

        // vignette and grain
        let mut rgb = to_rgb(&pm);
        let (cx, cy) = (w * 0.5, h * 0.5);
        // coarse grain (3 px cells) keeps the clip small
        let cw = self.w / 3 + 1;
        let mut grain = Rng::new(r.next());
        let cells: Vec<f32> = (0..cw * (self.h / 3 + 1)).map(|_| grain.f()).collect();
        for y in 0..self.h {
            for x in 0..self.w {
                let d = (((x as f32 - cx) / cx).powi(2) + ((y as f32 - cy) / cy).powi(2)).sqrt();
                let vig = 1.0 - 0.14 * (d - 0.5).max(0.0).powf(1.5);
                let i = ((y * self.w + x) * 3) as usize;
                let n = 1.0 + (cells[((y / 3) * cw + x / 3) as usize] - 0.5) * 0.04;
                let g = vig * n;
                // slightly cool cast, like old film base
                let tint = [0.985, 0.995, 1.0];
                for c in 0..3 {
                    rgb[i + c] = (rgb[i + c] as f32 * g * tint[c]).clamp(0.0, 255.0) as u8;
                }
            }
        }
        rgb
    }
}

// ------------------------------------------------------------------ burn

struct Blob {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r0: f32,
    age: u32,
    life: u32,
    hue: f32,
}

struct Burn {
    rng: Rng,
    w: u32,
    h: u32,
    blobs: Vec<Blob>,
    frame_no: u32,
    next_leak: u32,
    leak_left: u32,
    leak_len: u32,
    leak_pos: (f32, f32),
}

impl Burn {
    fn new(w: u32, h: u32) -> Self {
        Burn {
            rng: Rng::new(23),
            w,
            h,
            blobs: Vec::new(),
            frame_no: 0,
            next_leak: 40,
            leak_left: 0,
            leak_len: 6,
            leak_pos: (0.0, 0.0),
        }
    }
}

fn warm(t: f32) -> (f32, f32, f32) {
    // t = 0 dark red, 1 orange-yellow
    (1.0, 0.10 + 0.30 * t, 0.01 + 0.04 * t)
}

fn radial(pm: &mut Pixmap, x: f32, y: f32, rad: f32, stops: Vec<GradientStop>) {
    let Some(shader) =
        RadialGradient::new(Point::from_xy(x, y), 0.0, Point::from_xy(x, y), rad, stops, SpreadMode::Pad, Transform::identity())
    else {
        return;
    };
    let p = Paint { shader, blend_mode: BlendMode::Plus, ..Paint::default() };
    let (w, h) = (pm.width() as f32, pm.height() as f32);
    let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
    pm.fill_rect(rect, &p, Transform::identity(), None);
}

fn stop(pos: f32, c: (f32, f32, f32), a: f32) -> GradientStop {
    GradientStop::new(pos, Color::from_rgba(c.0, c.1, c.2, a.clamp(0.0, 1.0)).unwrap())
}

impl Layer for Burn {
    fn frame(&mut self) -> Vec<u8> {
        let (w, h) = (self.w as f32, self.h as f32);
        let r = &mut self.rng;
        let mut pm = Pixmap::new(self.w, self.h).unwrap();
        pm.fill(Color::BLACK);

        // keep 2..4 blobs alive
        let want = 2 + (self.frame_no / 53) % 3;
        while (self.blobs.len() as u32) < want {
            let edge = r.f() < 0.7;
            let (x, y) = if edge {
                match r.int(0, 3) {
                    0 => (r.range(0.0, w), -20.0),
                    1 => (r.range(0.0, w), h + 20.0),
                    2 => (-20.0, r.range(0.0, h)),
                    _ => (w + 20.0, r.range(0.0, h)),
                }
            } else {
                (r.range(0.0, w), r.range(0.0, h))
            };
            self.blobs.push(Blob {
                x,
                y,
                vx: r.range(-1.5, 1.5),
                vy: r.range(-1.5, 1.5),
                r0: r.range(60.0, 170.0),
                age: 0,
                life: r.int(15, 40),
                hue: r.f(),
            });
        }
        for b in &mut self.blobs {
            let t = b.age as f32 / b.life as f32;
            // grow, then burn out
            let size = b.r0 * (0.35 + 0.65 * (t * 3.0).min(1.0)) * (1.0 + 0.25 * t);
            let a = (t * std::f32::consts::PI).sin().powf(0.8) * 0.9;
            let core = warm(b.hue);
            radial(
                &mut pm,
                b.x,
                b.y,
                size,
                vec![stop(0.0, (1.0, 0.38 + 0.2 * b.hue, 0.08), a * 0.9), stop(0.35, core, a * 0.65), stop(0.7, (0.8, 0.05, 0.0), a * 0.25), stop(1.0, (0.5, 0.0, 0.0), 0.0)],
            );
            b.x += b.vx;
            b.y += b.vy;
            b.age += 1;
        }
        self.blobs.retain(|b| b.age < b.life);

        // light leak
        if self.leak_left == 0 && self.frame_no >= self.next_leak {
            self.leak_len = r.int(5, 8);
            self.leak_left = self.leak_len;
            self.leak_pos = (r.range(0.0, w), if r.f() < 0.5 { 0.0 } else { h });
            self.next_leak = self.frame_no + r.int(90, 120);
        }
        if self.leak_left > 0 {
            let t = 1.0 - self.leak_left as f32 / self.leak_len as f32;
            let a = (t * std::f32::consts::PI).sin().max(0.15);
            let (x, y) = self.leak_pos;
            radial(&mut pm, x, y, w * 1.1, vec![stop(0.0, (1.0, 0.72, 0.4), a * 0.85), stop(0.4, (1.0, 0.4, 0.12), a * 0.5), stop(1.0, (0.9, 0.2, 0.0), 0.0)]);
            self.leak_left -= 1;
        }

        // edges a little lighter
        let edge = r.range(0.85, 1.0);
        radial(&mut pm, w * 0.5, h * 0.5, w * 0.78, vec![stop(0.0, (0.0, 0.0, 0.0), 0.0), stop(0.7, (0.0, 0.0, 0.0), 0.0), stop(1.0, (0.55, 0.2, 0.05), 0.22 * edge)]);

        self.frame_no += 1;
        to_rgb(&pm)
    }
}

// ------------------------------------------------------------------ vhs static

struct Band {
    y: f32,
    h: f32,
    speed: f32,
    gain: f32,
}

struct VhsStatic {
    rng: Rng,
    w: usize,
    h: usize,
    bands: Vec<Band>,
    frame_no: u32,
    next_tear: u32,
    tear_left: u32,
    tear_h: usize,
}

impl VhsStatic {
    fn new(w: usize, h: usize) -> Self {
        let mut rng = Rng::new(37);
        let n = rng.int(2, 4);
        let bands = (0..n)
            .map(|_| Band { y: rng.range(0.0, h as f32), h: rng.range(4.0, 20.0), speed: rng.range(0.4, 1.6), gain: rng.range(0.6, 1.0) })
            .collect();
        VhsStatic { rng, w, h, bands, frame_no: 0, next_tear: 30, tear_left: 0, tear_h: 12 }
    }
}

impl Layer for VhsStatic {
    fn frame(&mut self) -> Vec<u8> {
        let (w, h) = (self.w, self.h);
        let r = &mut self.rng;
        let mut buf = vec![0f32; w * h * 3];

        // base noise: rows are correlated (each row has its own level), pixels vary within the row
        let mut row_level = 0.5f32;
        for y in 0..h {
            row_level = (row_level * 0.55 + r.f() * 0.45).clamp(0.0, 1.0);
            // more noise towards the bottom edge of the picture, like head switching noise
            let low = (y as f32 / h as f32).powi(3);
            let base = 0.01 + 0.06 * row_level * row_level + 0.10 * low * row_level;
            let row = &mut buf[y * w * 3..(y + 1) * w * 3];
            let mut run = 0.0f32;
            for x in 0..w {
                // short horizontal smear: value persists over a few pixels
                run = run * 0.6 + r.f() * 0.4;
                let v = base * (0.2 + 2.2 * run * r.f());
                let tint = [1.0 + r.range(-0.15, 0.15), 1.0, 1.0 + r.range(-0.15, 0.15)];
                for c in 0..3 {
                    row[x * 3 + c] = v * tint[c];
                }
            }
            // bright horizontal streaks
            let streaks = (r.f() * (0.5 + 3.0 * low)) as u32;
            for _ in 0..streaks {
                let len = r.int(8, 120) as usize;
                let x0 = r.int(0, (w - 1) as u32) as usize;
                let b = r.range(0.35, 1.0);
                for x in x0..(x0 + len).min(w) {
                    let f = b * (1.0 - (x - x0) as f32 / len as f32 * 0.6);
                    for c in 0..3 {
                        row[x * 3 + c] = row[x * 3 + c].max(f);
                    }
                }
            }
        }

        // interference bands drifting upwards
        for b in &mut self.bands {
            let y0 = b.y as isize;
            for dy in 0..b.h as isize {
                let y = (y0 + dy).rem_euclid(h as isize) as usize;
                let prof = (dy as f32 / b.h).clamp(0.0, 1.0);
                let amp = b.gain * 0.35 * (prof * std::f32::consts::PI).sin().max(0.2);
                let mut run = 0.0f32;
                for x in 0..w {
                    run = run * 0.7 + r.f() * 0.3;
                    let v = amp * (0.3 + run * 1.6) * r.f().max(0.35);
                    for c in 0..3 {
                        buf[(y * w + x) * 3 + c] += v;
                    }
                }
            }
            b.y -= b.speed;
            if b.y < -b.h {
                b.y += h as f32;
            }
        }

        // tracking tear at the bottom
        if self.tear_left == 0 && self.frame_no >= self.next_tear {
            self.tear_left = r.int(3, 6);
            self.tear_h = r.int(10, 30) as usize;
            self.next_tear = self.frame_no + r.int(60, 90);
        }
        if self.tear_left > 0 {
            let jitter = r.range(0.7, 1.0);
            for dy in 0..self.tear_h {
                let y = h - 1 - dy;
                let fall = 1.0 - dy as f32 / self.tear_h as f32 * 0.5;
                let shift = r.int(0, 40) as usize;
                let mut run = 0.0f32;
                for x in 0..w {
                    run = run * 0.8 + r.f() * 0.2;
                    let on = if (x + shift) % 97 < 70 { 1.0 } else { 0.35 };
                    let v = (0.25 + 1.3 * run * r.f().sqrt()) * fall * jitter * on;
                    for c in 0..3 {
                        let i = (y * w + x) * 3 + c;
                        buf[i] = buf[i].max(v.min(1.0));
                    }
                }
            }
            self.tear_left -= 1;
        }

        self.frame_no += 1;
        buf.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8).collect()
    }
}

// ------------------------------------------------------------------ output

fn encode(layer: &mut dyn Layer, w: u32, h: u32, fps: &str, frames: usize, crf: &str, out: &str) {
    let mut child = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s"])
        .arg(format!("{w}x{h}"))
        .args(["-r", fps, "-i", "-", "-c:v", "libx264", "-crf", crf, "-pix_fmt", "yuv420p", "-g", "30"])
        .args(["-movflags", "+faststart", "-map_metadata", "-1", "-fflags", "+bitexact", "-flags:v", "+bitexact"])
        .arg(out)
        .stdin(Stdio::piped())
        .spawn()
        .expect("start ffmpeg");
    let mut stdin = child.stdin.take().unwrap();

    // Frames 0..FADE are only generated and kept. The clip shows frames FADE..frames, then
    // FADE more frames that cross-fade from the continuation into the kept head, so the last
    // frame flows straight into the first one.
    let head: Vec<Vec<u8>> = (0..FADE).map(|_| layer.frame()).collect();
    for _ in FADE..frames {
        stdin.write_all(&layer.frame()).unwrap();
    }
    for (j, kept) in head.iter().enumerate() {
        let tail = layer.frame();
        let t = (j + 1) as f32 / (FADE + 1) as f32;
        let mixed: Vec<u8> = tail.iter().zip(kept).map(|(&a, &b)| (a as f32 * (1.0 - t) + b as f32 * t + 0.5) as u8).collect();
        stdin.write_all(&mixed).unwrap();
    }
    drop(stdin);
    assert!(child.wait().unwrap().success(), "ffmpeg failed for {out}");
    println!("wrote {out}");
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "assets/video".into());
    let only = std::env::args().nth(2);
    let want = |n: &str| only.as_deref().is_none_or(|o| o == n);
    if want("dust") {
        encode(&mut Dust::new(720, 720), 720, 720, "30", 230, "24", &format!("{dir}/dust.mp4"));
    }
    if want("burn") {
        encode(&mut Burn::new(720, 720), 720, 720, "30", 230, "18", &format!("{dir}/burn.mp4"));
    }
    if want("vhs-static") {
        encode(&mut VhsStatic::new(1000, 750), 1000, 750, "30000/1001", 293, "30", &format!("{dir}/vhs-static.mp4"));
    }
}
