//! Minimal 2D text canvas: fill/stroke text with TrueType fonts, similar to a
//! browser canvas with `textBaseline = 'top'`.

use std::sync::Arc;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// Font file bytes plus face index.
#[derive(Clone)]
pub struct FontData {
    pub data: Arc<Vec<u8>>,
    pub index: u32,
}

impl FontData {
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self { data: Arc::new(bytes.to_vec()), index: 0 }
    }

    fn face(&self) -> Option<ttf_parser::Face<'_>> {
        ttf_parser::Face::parse(&self.data, self.index).ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    pub fn parse(s: &str) -> Self {
        match s {
            "center" => Align::Center,
            "right" => Align::Right,
            _ => Align::Left,
        }
    }
}

/// A font chain: the first font that has a glyph for a character is used.
pub struct Fonts<'a>(pub &'a [FontData]);

struct Glyphs<'a> {
    faces: Vec<ttf_parser::Face<'a>>,
}

impl<'a> Glyphs<'a> {
    fn new(fonts: &'a [FontData]) -> Self {
        Self { faces: fonts.iter().filter_map(|f| f.face()).collect() }
    }

    fn lookup(&self, c: char) -> Option<(usize, ttf_parser::GlyphId)> {
        for (i, f) in self.faces.iter().enumerate() {
            if let Some(g) = f.glyph_index(c) {
                return Some((i, g));
            }
        }
        // missing everywhere: .notdef of the primary font
        (!self.faces.is_empty()).then_some((0, ttf_parser::GlyphId(0)))
    }

    fn scale(&self, i: usize, size: f32) -> f32 {
        size / self.faces[i].units_per_em() as f32
    }

    /// Distance from the top of the em box to the baseline, in px.
    fn top_to_baseline(&self, size: f32) -> f32 {
        let Some(f) = self.faces.first() else { return size * 0.8 };
        let asc = f.ascender() as f32;
        let desc = f.descender() as f32;
        if asc - desc <= 0.0 {
            return size * 0.8;
        }
        size * asc / (asc - desc)
    }

    fn kern(&self, i: usize, a: ttf_parser::GlyphId, b: ttf_parser::GlyphId) -> f32 {
        let Some(kern) = self.faces[i].tables().kern else { return 0.0 };
        for st in kern.subtables {
            if st.horizontal && !st.variable {
                if let Some(v) = st.glyphs_kerning(a, b) {
                    return v as f32;
                }
            }
        }
        0.0
    }

    /// (face, glyph, x offset in px) for each char, plus total advance.
    fn layout(&self, text: &str, size: f32) -> (Vec<(usize, ttf_parser::GlyphId, f32)>, f32) {
        let mut x = 0.0;
        let mut out = Vec::new();
        let mut prev: Option<(usize, ttf_parser::GlyphId)> = None;
        for c in text.chars() {
            let Some((i, g)) = self.lookup(c) else { continue };
            let s = self.scale(i, size);
            if let Some((pi, pg)) = prev {
                if pi == i {
                    x += self.kern(i, pg, g) * s;
                }
            }
            out.push((i, g, x));
            x += self.faces[i].glyph_hor_advance(g).unwrap_or(0) as f32 * s;
            prev = Some((i, g));
        }
        (out, x)
    }
}

struct Outline<'p> {
    pb: &'p mut PathBuilder,
    ox: f32,
    oy: f32,
    s: f32,
}

impl ttf_parser::OutlineBuilder for Outline<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.pb.move_to(self.ox + x * self.s, self.oy - y * self.s);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.pb.line_to(self.ox + x * self.s, self.oy - y * self.s);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.pb.quad_to(self.ox + x1 * self.s, self.oy - y1 * self.s, self.ox + x * self.s, self.oy - y * self.s);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.pb.cubic_to(
            self.ox + x1 * self.s,
            self.oy - y1 * self.s,
            self.ox + x2 * self.s,
            self.oy - y2 * self.s,
            self.ox + x * self.s,
            self.oy - y * self.s,
        );
    }
    fn close(&mut self) {
        self.pb.close();
    }
}

pub struct TextCanvas {
    pub pixmap: Pixmap,
}

impl TextCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self { pixmap: Pixmap::new(width.max(1), height.max(1)).expect("canvas size") }
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }
    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    pub fn clear(&mut self) {
        self.pixmap.fill(tiny_skia::Color::TRANSPARENT);
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, rgba: [u8; 4]) {
        if let Some(r) = tiny_skia::Rect::from_xywh(x, y, w, h) {
            let mut p = Paint::default();
            p.set_color_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
            p.anti_alias = false;
            self.pixmap.fill_rect(r, &p, Transform::identity(), None);
        }
    }

    /// Clear a rectangle to transparent (like `clearRect`).
    pub fn clear_rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        let (cw, ch) = (self.width(), self.height());
        let data = self.pixmap.data_mut();
        for yy in y..(y + h).min(ch) {
            for xx in x..(x + w).min(cw) {
                let i = ((yy * cw + xx) * 4) as usize;
                data[i..i + 4].fill(0);
            }
        }
    }

    pub fn measure(fonts: &[FontData], size: f32, text: &str) -> f32 {
        Glyphs::new(fonts).layout(text, size).1
    }

    /// Draw one line of text. `y` is the top of the em box.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &mut self,
        fonts: &[FontData],
        size: f32,
        text: &str,
        x: f32,
        y: f32,
        align: Align,
        fill: Option<[u8; 3]>,
        stroke: Option<([u8; 3], f32)>,
        anti_alias: bool,
    ) {
        let g = Glyphs::new(fonts);
        if g.faces.is_empty() || text.is_empty() {
            return;
        }
        let (glyphs, width) = g.layout(text, size);
        let x0 = match align {
            Align::Left => x,
            Align::Center => x - width / 2.0,
            Align::Right => x - width,
        };
        let baseline = y + g.top_to_baseline(size);
        let mut pb = PathBuilder::new();
        for (i, gid, gx) in glyphs {
            let mut o = Outline { pb: &mut pb, ox: x0 + gx, oy: baseline, s: g.scale(i, size) };
            g.faces[i].outline_glyph(gid, &mut o);
        }
        let Some(path) = pb.finish() else { return };
        if let Some((c, w)) = stroke {
            let mut p = Paint::default();
            p.set_color_rgba8(c[0], c[1], c[2], 255);
            p.anti_alias = anti_alias;
            let st = Stroke { width: w, miter_limit: 10.0, ..Default::default() };
            self.pixmap.stroke_path(&path, &p, &st, Transform::identity(), None);
        }
        if let Some(c) = fill {
            let mut p = Paint::default();
            p.set_color_rgba8(c[0], c[1], c[2], 255);
            p.anti_alias = anti_alias;
            self.pixmap.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
        }
    }

    /// Straight-alpha RGBA rows, top row first.
    pub fn to_rgba_straight(&self) -> Vec<u8> {
        let mut out = self.pixmap.data().to_vec();
        for px in out.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a != 0 && a != 255 {
                for c in &mut px[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        out
    }

    /// Greedy word wrap like the reference caption layout.
    pub fn wrap(fonts: &[FontData], size: f32, text: &str, max_width: f32) -> Vec<String> {
        let g = Glyphs::new(fonts);
        let mut lines = Vec::new();
        for para in text.split('\n') {
            if para.is_empty() {
                lines.push(String::new());
                continue;
            }
            let mut words = para.split(' ');
            let mut cur = words.next().unwrap_or("").to_string();
            for w in words {
                let test = format!("{cur} {w}");
                if g.layout(&test, size).1 > max_width {
                    lines.push(std::mem::replace(&mut cur, w.to_string()));
                } else {
                    cur = test;
                }
            }
            lines.push(cur);
        }
        lines
    }
}
