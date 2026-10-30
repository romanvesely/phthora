//! Button symbols drawn as shapes. Symbol characters come from whatever fallback font the
//! system has, each with its own baseline and width; drawn shapes sit exactly in the middle of
//! their button and have the same size everywhere.

use eframe::egui::{self, Atom, Color32, Pos2, Rect, Shape, Stroke, Vec2};
use std::f32::consts::{FRAC_1_SQRT_2, PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Menu,
    Play,
    Pause,
    Prev,
    Next,
    /// closed fold, points right
    Expand,
    /// open fold, points down
    Collapse,
    Up,
    Down,
    Close,
    Duplicate,
    Reset,
    Refresh,
    Pencil,
    Lock,
    Unlock,
    Mute,
    Unmute,
    Settings,
    Fullscreen,
    Ungroup,
    Record,
    Stop,
    Modulator,
    Plus,
    Warning,
}

#[cfg(test)]
impl Icon {
    pub const ALL: [Icon; 26] = [
        Icon::Menu,
        Icon::Play,
        Icon::Pause,
        Icon::Prev,
        Icon::Next,
        Icon::Expand,
        Icon::Collapse,
        Icon::Up,
        Icon::Down,
        Icon::Close,
        Icon::Duplicate,
        Icon::Reset,
        Icon::Refresh,
        Icon::Pencil,
        Icon::Lock,
        Icon::Unlock,
        Icon::Mute,
        Icon::Unmute,
        Icon::Settings,
        Icon::Fullscreen,
        Icon::Ungroup,
        Icon::Record,
        Icon::Stop,
        Icon::Modulator,
        Icon::Plus,
        Icon::Warning,
    ];
}

/// Side of the drawn symbol in small buttons and in normal buttons.
pub const SMALL: f32 = 10.0;
pub const NORMAL: f32 = 12.0;
/// Height of small icon buttons; the same for every one of them.
const SMALL_H: f32 = 18.0;


/// The symbol as an atom, painted in the button's text color (so it follows hover, selection
/// and disabled state like text does), or in `tint`.
pub fn atom(icon: Icon, side: f32, tint: Option<Color32>) -> Atom<'static> {
    Atom::paint(Vec2::splat(side), move |ui, args| {
        paint(ui.painter(), icon, args.rect, tint.unwrap_or(args.fallback_text_color));
    })
}

/// A small square icon button, as used in effect headers and lists.
pub fn small(icon: Icon) -> IconButton {
    IconButton::new(icon, SMALL, Vec2::splat(SMALL_H))
}

pub fn small_tinted(icon: Icon, tint: Option<Color32>) -> IconButton {
    IconButton { tint, ..small(icon) }
}

/// An icon button of normal height, as used in the top bar.
pub fn button(ui: &egui::Ui, icon: Icon) -> IconButton {
    let h = ui.spacing().interact_size.y;
    IconButton::new(icon, NORMAL, Vec2::new(h + 6.0, h))
}

/// The same as a [`egui::Button`] (for menus), with the symbol in it.
pub fn menu_button(ui: &egui::Ui, icon: Icon) -> egui::Button<'static> {
    let h = ui.spacing().interact_size.y;
    egui::Button::new(atom(icon, NORMAL, None)).min_size(Vec2::new(h + 6.0, h))
}

/// Icon with a label after it, e.g. the record button.
pub fn with_text(icon: Icon, text: impl Into<egui::WidgetText>, tint: Option<Color32>) -> egui::Button<'static> {
    egui::Button::new((atom(icon, SMALL, tint), Atom::from(text.into())))
}

/// A frameless symbol, e.g. a drag handle.
pub fn bare(icon: Icon, sense: egui::Sense) -> IconButton {
    IconButton { frame: false, sense, ..small(icon) }
}

/// Fixed-size button that paints its symbol in the middle. egui's own buttons size themselves
/// from the text padding, which makes a symbol-only button wider than high.
pub struct IconButton {
    icon: Icon,
    side: f32,
    size: Vec2,
    selected: bool,
    tint: Option<Color32>,
    frame: bool,
    sense: egui::Sense,
}

impl IconButton {
    fn new(icon: Icon, side: f32, size: Vec2) -> Self {
        Self { icon, side, size, selected: false, tint: None, frame: true, sense: egui::Sense::click() }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl egui::Widget for IconButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(self.size, self.sense);
        if ui.is_rect_visible(rect) {
            let vis = ui.style().interact_selectable(&resp, self.selected);
            if self.frame {
                ui.painter().rect(rect.expand(vis.expansion), vis.corner_radius, vis.weak_bg_fill, vis.bg_stroke, egui::StrokeKind::Inside);
            }
            // a disabled ui fades everything its painter draws
            let color = self.tint.unwrap_or(vis.fg_stroke.color);
            paint(ui.painter(), self.icon, Rect::from_center_size(rect.center(), Vec2::splat(self.side)), color);
        }
        let selected = self.selected;
        resp.widget_info(|| {
            let mut info = egui::WidgetInfo::new(egui::Role::Button);
            info.enabled = ui.is_enabled();
            info.selected = Some(selected);
            info
        });
        resp
    }
}

/// Paint `icon` centred in `rect`, scaled to its shorter side.
pub fn paint(p: &egui::Painter, icon: Icon, rect: Rect, color: Color32) {
    p.extend(shapes(icon, rect, color));
}

fn shapes(icon: Icon, rect: Rect, color: Color32) -> Vec<Shape> {
    let c = rect.center();
    let u = rect.width().min(rect.height()) / 2.0;
    let w = (u * 0.3).clamp(1.2, 2.0);
    let stroke = Stroke::new(w, color);
    // local coordinates in units of u, y down
    let at = |x: f32, y: f32| Pos2::new(c.x + x * u, c.y + y * u);
    let poly = |pts: &[(f32, f32)]| Shape::convex_polygon(pts.iter().map(|&(x, y)| at(x, y)).collect(), color, Stroke::NONE);
    let line = |pts: &[(f32, f32)]| Shape::line(pts.iter().map(|&(x, y)| at(x, y)).collect(), stroke);
    // arc around (cx, cy) with radius r from angle a0 to a1 (radians, y down: positive = clockwise)
    let arc = |cx: f32, cy: f32, r: f32, a0: f32, a1: f32| -> Vec<(f32, f32)> {
        let n = ((a1 - a0).abs() / TAU * 32.0).ceil().max(2.0) as usize;
        (0..=n).map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            (cx + r * a.cos(), cy + r * a.sin())
        }).collect()
    };
    // a triangle pointing in direction (dx, dy), the visual weight slightly behind the tip
    let tri = |dx: f32, dy: f32, len: f32, half: f32| {
        let (px, py) = (-dy, dx);
        let back = -0.5 * len;
        let tip = 0.5 * len;
        poly(&[(dx * tip, dy * tip), (dx * back + px * half, dy * back + py * half), (dx * back - px * half, dy * back - py * half)])
    };
    let mut shapes: Vec<Shape> = Vec::new();
    match icon {
        Icon::Menu => {
            for y in [-0.6, 0.0, 0.6] {
                shapes.push(line(&[(-0.8, y), (0.8, y)]));
            }
        }
        Icon::Play | Icon::Next | Icon::Expand => shapes.push(tri(1.0, 0.0, if icon == Icon::Play { 1.5 } else { 1.3 }, if icon == Icon::Play { 0.85 } else { 0.75 })),
        Icon::Prev => shapes.push(tri(-1.0, 0.0, 1.3, 0.75)),
        Icon::Collapse => shapes.push(tri(0.0, 1.0, 1.3, 0.75)),
        Icon::Up => shapes.push(tri(0.0, -1.0, 1.0, 0.65)),
        Icon::Down => shapes.push(tri(0.0, 1.0, 1.0, 0.65)),
        Icon::Pause => {
            for x in [-0.45, 0.45] {
                shapes.push(poly(&[(x - 0.22, -0.75), (x + 0.22, -0.75), (x + 0.22, 0.75), (x - 0.22, 0.75)]));
            }
        }
        Icon::Stop => shapes.push(poly(&[(-0.65, -0.65), (0.65, -0.65), (0.65, 0.65), (-0.65, 0.65)])),
        Icon::Record => shapes.push(Shape::circle_filled(c, 0.75 * u, color)),
        Icon::Close => {
            shapes.push(line(&[(-0.65, -0.65), (0.65, 0.65)]));
            shapes.push(line(&[(-0.65, 0.65), (0.65, -0.65)]));
        }
        Icon::Plus => {
            shapes.push(line(&[(-0.75, 0.0), (0.75, 0.0)]));
            shapes.push(line(&[(0.0, -0.75), (0.0, 0.75)]));
        }
        Icon::Duplicate => {
            // the back sheet only where the front one does not cover it
            shapes.push(line(&[(-0.3, 0.3), (-0.85, 0.3), (-0.85, -0.85), (0.3, -0.85), (0.3, -0.3)]));
            shapes.push(line(&[(-0.3, -0.3), (0.85, -0.3), (0.85, 0.85), (-0.3, 0.85), (-0.3, -0.3)]));
        }
        Icon::Reset | Icon::Refresh => {
            // circle open at the top on one side, the arrow head at the top pointing along the arc
            let s = if icon == Icon::Refresh { 1.0 } else { -1.0 };
            let pts: Vec<(f32, f32)> = arc(0.0, 0.0, 0.7, 0.0, 1.5 * PI).into_iter().map(|(x, y)| (s * x, y)).collect();
            shapes.push(line(&pts));
            let (ex, ey) = (0.0, -0.7);
            shapes.push(poly(&[(ex + s * 0.45, ey), (ex - s * 0.1, ey - 0.38), (ex - s * 0.1, ey + 0.38)]));
        }
        Icon::Pencil => {
            // along the diagonal from lower left (tip) to upper right
            let (dx, dy) = (FRAC_1_SQRT_2, -FRAC_1_SQRT_2);
            let (nx, ny) = (FRAC_1_SQRT_2, FRAC_1_SQRT_2);
            let pt = |s: f32, w: f32| (s * dx + w * nx, s * dy + w * ny);
            shapes.push(poly(&[pt(-0.35, -0.3), pt(1.05, -0.3), pt(1.05, 0.3), pt(-0.35, 0.3)]));
            shapes.push(poly(&[pt(-0.5, -0.3), pt(-0.5, 0.3), pt(-1.1, 0.0)]));
        }
        Icon::Lock | Icon::Unlock => {
            shapes.push(poly(&[(-0.7, -0.1), (0.7, -0.1), (0.7, 0.85), (-0.7, 0.85)]));
            let lift = if icon == Icon::Unlock { 0.25 } else { 0.0 };
            let top = -0.4 - lift;
            let mut pts = vec![(-0.42, -0.1)];
            pts.extend(arc(0.0, top, 0.42, PI, TAU));
            if icon == Icon::Lock {
                pts.push((0.42, -0.1));
            }
            shapes.push(line(&pts));
        }
        Icon::Mute | Icon::Unmute => {
            let x0 = -0.35;
            shapes.push(poly(&[(x0 - 0.5, -0.3), (x0, -0.3), (x0, 0.3), (x0 - 0.5, 0.3)]));
            shapes.push(poly(&[(x0, -0.3), (x0 + 0.5, -0.8), (x0 + 0.5, 0.8), (x0, 0.3)]));
            if icon == Icon::Unmute {
                for r in [0.4, 0.75] {
                    shapes.push(line(&arc(x0 + 0.5, 0.0, r, -0.9, 0.9)));
                }
            } else {
                shapes.push(line(&[(0.45, -0.3), (1.0, 0.3)]));
                shapes.push(line(&[(0.45, 0.3), (1.0, -0.3)]));
            }
        }
        Icon::Settings => {
            shapes.push(Shape::circle_stroke(c, 0.5 * u, Stroke::new(0.32 * u, color)));
            for k in 0..8 {
                let a = k as f32 * TAU / 8.0;
                let (dx, dy) = (a.cos(), a.sin());
                let (nx, ny) = (-dy, dx);
                let h = 0.17;
                let q = |r: f32, s: f32| (dx * r + nx * s, dy * r + ny * s);
                shapes.push(poly(&[q(0.55, -h), q(0.95, -h), q(0.95, h), q(0.55, h)]));
            }
        }
        Icon::Fullscreen => {
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                shapes.push(line(&[(sx * 0.8, sy * 0.35), (sx * 0.8, sy * 0.8), (sx * 0.35, sy * 0.8)]));
            }
        }
        Icon::Ungroup => {
            shapes.push(line(&[(-0.8, -0.7), (0.8, -0.7), (0.8, 0.7), (-0.8, 0.7), (-0.8, -0.7)]));
            shapes.push(line(&[(-0.45, 0.0), (0.45, 0.0)]));
        }
        Icon::Modulator => {
            let pts: Vec<(f32, f32)> = (0..=16).map(|i| {
                let x = -0.85 + 1.7 * i as f32 / 16.0;
                (x, -0.45 * (x / 0.85 * PI).sin())
            }).collect();
            shapes.push(line(&pts));
        }
        Icon::Warning => {
            shapes.push(line(&[(0.0, -0.85), (0.9, 0.75), (-0.9, 0.75), (0.0, -0.85)]));
            shapes.push(line(&[(0.0, -0.3), (0.0, 0.2)]));
            shapes.push(Shape::circle_filled(at(0.0, 0.48), w * 0.6, color));
        }
    }
    shapes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_paints_inside_its_rect() {
        let rect = Rect::from_center_size(Pos2::new(50.0, 50.0), Vec2::splat(SMALL));
        for icon in Icon::ALL {
            let shapes = shapes(icon, rect, Color32::WHITE);
            assert!(!shapes.is_empty(), "{icon:?} paints nothing");
            let mut bb = Rect::NOTHING;
            for s in &shapes {
                bb = bb.union(s.visual_bounding_rect());
            }
            // inside the rect (the stroke may reach a little over), and roughly centred
            assert!(rect.expand(1.5).contains_rect(bb), "{icon:?} leaves its rect: {bb:?}");
            let off = bb.center() - rect.center();
            assert!(off.x.abs() <= 1.6 && off.y.abs() <= 1.6, "{icon:?} off centre by {off:?}");
        }
    }

    #[test]
    fn small_icon_buttons_have_one_size() {
        let ctx = egui::Context::default();
        let mut sizes = Vec::new();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            for icon in Icon::ALL {
                sizes.push(ui.add(small(icon)).rect.size());
            }
        });
        out.textures_delta.clear();
        assert!(sizes.windows(2).all(|w| w[0] == w[1]), "{sizes:?}");
    }
}
