//! Canvases for the drawing effects: Push Draw paints a displacement field, Mask Draw a
//! mask. Brush dabs are soft discs whose radius grows with the pointer speed. The canvas
//! is stored in the effect as a PNG data URL (`imageData`), so presets keep the drawing.

use base64::Engine as _;
use tiny_skia::{Color, FilterQuality, GradientStop, Paint, Pixmap, PixmapPaint, Point, RadialGradient, Rect, SpreadMode, Transform};

/// Effect data key holding the canvas as a PNG data URL.
pub const DATA_KEY: &str = "imageData";
/// Brush steps per second (radius and push strength are per step, as in a 60 Hz loop).
pub const DAB_RATE: f64 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintKind {
    Push,
    Mask,
}

impl PaintKind {
    pub fn of(effect_id: &str) -> Option<Self> {
        match effect_id {
            "push_draw" => Some(PaintKind::Push),
            "mask_draw" => Some(PaintKind::Mask),
            _ => None,
        }
    }

    /// Empty canvas: no displacement (0.5, 0.5) or no mask.
    fn clear_color(self) -> Color {
        match self {
            PaintKind::Push => Color::from_rgba8(128, 128, 0, 255),
            PaintKind::Mask => Color::BLACK,
        }
    }
}

pub struct PaintCanvas {
    pub kind: PaintKind,
    pixmap: Pixmap,
    /// changes with every edit, so renders know when to upload
    pub version: u64,
    /// the data URL these pixels were last saved to or loaded from
    pub synced: String,
}

impl PaintCanvas {
    pub fn new(kind: PaintKind, w: u32, h: u32) -> Self {
        let mut c = Self { kind, pixmap: Pixmap::new(w.max(1), h.max(1)).unwrap(), version: 0, synced: String::new() };
        c.clear();
        c
    }

    pub fn size(&self) -> (u32, u32) {
        (self.pixmap.width(), self.pixmap.height())
    }

    pub fn clear(&mut self) {
        self.pixmap.fill(self.kind.clear_color());
        self.version += 1;
    }

    /// Scale the drawing to a new canvas size.
    pub fn resize(&mut self, w: u32, h: u32) {
        if self.size() == (w, h) {
            return;
        }
        let old = std::mem::replace(&mut self.pixmap, Pixmap::new(w.max(1), h.max(1)).unwrap());
        self.pixmap.fill(self.kind.clear_color());
        self.draw_scaled(&old);
        self.version += 1;
    }

    fn draw_scaled(&mut self, src: &Pixmap) {
        let sx = self.pixmap.width() as f32 / src.width() as f32;
        let sy = self.pixmap.height() as f32 / src.height() as f32;
        let paint = PixmapPaint { quality: FilterQuality::Bilinear, ..Default::default() };
        self.pixmap.draw_pixmap(0, 0, src.as_ref(), &paint, Transform::from_scale(sx, sy), None);
    }

    /// One brush step at (x, y) in canvas pixels (top-left origin). `step` is the pointer
    /// movement since the previous step: it sets the radius and, for Push, the direction.
    pub fn dab(&mut self, x: f32, y: f32, step: (f32, f32), erase: bool) {
        let dist = (step.0 * step.0 + step.1 * step.1).sqrt();
        let radius = dist + 20.0 * self.pixmap.width() as f32 / 2000.0;
        let (rgb, alpha) = match self.kind {
            PaintKind::Push => {
                // movement opposite to the stroke, 30 levels per pixel around 128
                let ch = |v: f32| ((-v * 30.0 + 128.0).clamp(0.0, 255.0)) / 255.0;
                ([ch(step.0), ch(step.1), 0.0], 0.1)
            }
            PaintKind::Mask if erase => ([0.0; 3], 0.2),
            PaintKind::Mask => ([1.0, 0.0, 0.0], 0.2),
        };
        let col = |a: f32| Color::from_rgba(rgb[0], rgb[1], rgb[2], a).unwrap();
        let stops = vec![GradientStop::new(0.0, col(alpha)), GradientStop::new(1.0, col(0.0))];
        let Some(shader) = RadialGradient::new(Point::from_xy(x, y), radius * 0.66, Point::from_xy(x, y), radius, stops, SpreadMode::Pad, Transform::identity()) else {
            return;
        };
        let paint = Paint { shader, anti_alias: false, ..Default::default() };
        // outside the radius the gradient is transparent: only the disc's box changes
        if let Some(r) = Rect::from_xywh(x - radius - 1.0, y - radius - 1.0, 2.0 * radius + 2.0, 2.0 * radius + 2.0) {
            self.pixmap.fill_rect(r, &paint, Transform::identity(), None);
            self.version += 1;
        }
    }

    /// Straight RGBA8, top-down.
    pub fn rgba(&self) -> Vec<u8> {
        self.pixmap
            .pixels()
            .iter()
            .flat_map(|p| {
                let c = p.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            })
            .collect()
    }

    /// The canvas as a PNG data URL.
    pub fn to_data_url(&self) -> String {
        let png = self.pixmap.encode_png().unwrap_or_default();
        format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
    }

    /// Replace the drawing with a data URL's image, scaled to the canvas. An empty or
    /// unreadable URL leaves an empty canvas (returns false if it was unreadable).
    pub fn load_data_url(&mut self, url: &str) -> bool {
        self.pixmap.fill(self.kind.clear_color());
        self.version += 1;
        self.synced = url.to_string();
        if url.is_empty() {
            return true;
        }
        let b64 = url.split_once(',').map(|(_, d)| d).unwrap_or(url);
        let Ok(png) = base64::engine::general_purpose::STANDARD.decode(b64.trim()) else { return false };
        let Ok(img) = Pixmap::decode_png(&png) else { return false };
        self.draw_scaled(&img);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(c: &PaintCanvas, x: u32, y: u32) -> [u8; 4] {
        let d = c.rgba();
        let i = ((y * c.size().0 + x) * 4) as usize;
        [d[i], d[i + 1], d[i + 2], d[i + 3]]
    }

    #[test]
    fn mask_dab_is_a_soft_red_disc() {
        let mut c = PaintCanvas::new(PaintKind::Mask, 200, 100);
        // radius at rest: 20 * 200 / 2000 = 2 px; inner 66 % at full brush alpha
        c.dab(100.0, 50.0, (0.0, 0.0), false);
        // centre pixel (100.5, 50.5) is 0.71 px away, inside 1.32: red at alpha 0.2 over black
        assert_eq!(px(&c, 100, 50), [51, 0, 0, 255]);
        assert_eq!(px(&c, 110, 50), [0, 0, 0, 255], "outside the radius");
        for _ in 0..30 {
            c.dab(100.0, 50.0, (0.0, 0.0), false);
        }
        assert!(px(&c, 100, 50)[0] > 240, "repeated steps build up");
        for _ in 0..30 {
            c.dab(100.0, 50.0, (0.0, 0.0), true);
        }
        assert!(px(&c, 100, 50)[0] < 15, "erase paints black");
    }

    #[test]
    fn push_dab_encodes_the_movement() {
        let mut c = PaintCanvas::new(PaintKind::Push, 200, 100);
        assert_eq!(px(&c, 5, 5), [128, 128, 0, 255]);
        // moving right by 2 px per step: red channel towards 128 - 60, green unchanged
        // (8-bit blending at alpha 0.1 stops a few levels short)
        for _ in 0..60 {
            c.dab(100.0, 50.0, (2.0, 0.0), false);
        }
        let p = px(&c, 100, 50);
        assert!((68..=74).contains(&p[0]) && (p[1] as i32 - 128).abs() <= 1, "{p:?}");
        // a fast stroke has a bigger brush: 4 px radius here
        assert!(px(&c, 103, 50)[0] < 120);
    }

    #[test]
    fn data_url_roundtrip_and_resize() {
        let mut c = PaintCanvas::new(PaintKind::Mask, 64, 32);
        for _ in 0..20 {
            c.dab(20.0, 10.0, (1.0, 0.0), false);
        }
        let url = c.to_data_url();
        assert!(url.starts_with("data:image/png;base64,"));
        let mut d = PaintCanvas::new(PaintKind::Mask, 64, 32);
        assert!(d.load_data_url(&url));
        assert_eq!(d.rgba(), c.rgba());
        assert_eq!(d.synced, url);
        // a different canvas size scales the drawing
        let mut e = PaintCanvas::new(PaintKind::Mask, 128, 64);
        e.load_data_url(&url);
        assert!(px(&e, 40, 20)[0] > 100 && px(&e, 100, 50)[0] == 0);
        c.resize(128, 64);
        assert!(px(&c, 40, 20)[0] > 100);
        assert!(!d.load_data_url("data:image/png;base64,!!"));
        assert_eq!(px(&d, 20, 10), [0, 0, 0, 255], "unreadable data leaves an empty canvas");
    }
}
