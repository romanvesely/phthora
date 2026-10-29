//! Widgets for effect parameters and their modulators.

use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};
use phthora_core::modulator::{keyframe_value, Band, Modulator, Shape};
use phthora_core::params::{ColorStop, ParamKind, Value};
use phthora_core::{EffectInstance, OutputMode};
use std::collections::{BTreeMap, HashSet};

pub enum Action {
    /// a "Load..." button: pick a file for the effect's `path`
    PickFile { uid: u64 },
}

pub struct ParamCtx<'a> {
    pub mode: OutputMode,
    /// modulated values of this frame (to show what the modulators do)
    pub live: Option<&'a BTreeMap<String, Value>>,
    pub playhead: f64,
    pub actions: &'a mut Vec<Action>,
    pub open_mods: &'a mut HashSet<(u64, String)>,
    /// user audio bands defined in the audio settings
    pub custom_bands: usize,
}

const LABEL_W: f32 = 92.0;

fn label(ui: &mut egui::Ui, text: &str) {
    ui.add_sized([LABEL_W, 18.0], egui::Label::new(egui::RichText::new(text).small()).truncate());
}

fn is_disabled(e: &EffectInstance, cond: &Option<String>) -> bool {
    cond.as_ref().map(|k| e.get(k).map(|v| v.as_bool()).unwrap_or(false)).unwrap_or(false)
}

/// Draw all parameters of one effect. Returns true if anything changed.
pub fn effect_params(ui: &mut egui::Ui, e: &mut EffectInstance, cx: &mut ParamCtx) -> bool {
    let mut changed = false;
    let uid = e.uid;
    for p in e.desc.params.clone() {
        if p.hidden {
            continue;
        }
        let key = p.key.clone();
        match &p.kind {
            ParamKind::Float { min, max, step, image_only, disable_when, .. } => {
                if *image_only && cx.mode != OutputMode::Image {
                    continue;
                }
                let modulated = cx.mode == OutputMode::Video && e.modulators.get(&key).map(|m| m.enabled).unwrap_or(false);
                let mut v = e.f(&key);
                let disabled = is_disabled(e, disable_when);
                ui.horizontal(|ui| {
                    label(ui, &p.label);
                    // right to left: modulator toggle, number field, then the slider fills the rest
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !*image_only {
                            let open = cx.open_mods.contains(&(uid, key.clone()));
                            let on = e.modulators.get(&key).map(|m| m.enabled).unwrap_or(false);
                            let txt = egui::RichText::new("~").strong().color(if on { Color32::from_rgb(120, 220, 160) } else { ui.visuals().weak_text_color() });
                            if ui.add(egui::Button::new(txt).selected(open).small()).on_hover_text("Modulator").clicked() {
                                if open {
                                    cx.open_mods.remove(&(uid, key.clone()));
                                } else {
                                    cx.open_mods.insert((uid, key.clone()));
                                }
                            }
                        }
                        let speed = ((*max - *min) as f64 / 300.0).max(if *step > 0.0 { *step as f64 / 4.0 } else { 0.0 });
                        let mut live_v = match cx.live.and_then(|l| l.get(&key)) {
                            Some(Value::Float(lv)) if modulated => *lv,
                            _ => v,
                        };
                        let mut d = if modulated { egui::DragValue::new(&mut live_v) } else { egui::DragValue::new(&mut v) };
                        d = d.range(*min..=*max).speed(speed);
                        d = if *step >= 1.0 { d.fixed_decimals(0) } else { d.max_decimals(3) };
                        let rd = ui.add_enabled_ui(!disabled && !modulated, |ui| ui.add_sized([58.0, 18.0], d)).inner;
                        ui.spacing_mut().slider_width = (ui.available_width() - 4.0).max(40.0);
                        let mut s = egui::Slider::new(&mut v, *min..=*max).show_value(false);
                        if *step >= 1.0 {
                            s = s.integer();
                        } else if *step > 0.0 {
                            s = s.step_by(*step as f64);
                        }
                        let r = ui.add_enabled(!disabled && !modulated, s);
                        if r.changed() || rd.changed() {
                            changed = true;
                        }
                        if r.double_clicked() {
                            e.reset(&key);
                            v = e.f(&key);
                            changed = true;
                        }
                    });
                });
                if modulated
                    && let Some(Value::Float(lv)) = cx.live.and_then(|l| l.get(&key)) {
                        ui.horizontal(|ui| {
                            ui.add_space(LABEL_W + 8.0);
                            let frac = if max > min { (lv - min) / (max - min) } else { 0.0 };
                            ui.add(egui::ProgressBar::new(frac.clamp(0.0, 1.0)).desired_height(4.0).desired_width(ui.available_width() - 30.0));
                        });
                    }
                if !modulated && (v - e.f(&key)).abs() > f32::EPSILON {
                    e.set(&key, Value::Float(v));
                }
                if cx.open_mods.contains(&(uid, key.clone())) {
                    let m = e.modulators.entry(key.clone()).or_default();
                    if modulator_ui(ui, m, ui.make_persistent_id((uid, &key)), cx.playhead, cx.custom_bands) {
                        changed = true;
                    }
                }
            }
            ParamKind::Bool { .. } => {
                let mut b = e.get(&key).map(|v| v.as_bool()).unwrap_or(false);
                ui.horizontal(|ui| {
                    label(ui, &p.label);
                    if ui.checkbox(&mut b, "").changed() {
                        e.set(&key, Value::Bool(b));
                        changed = true;
                    }
                });
            }
            ParamKind::Combo { values, .. } => {
                let cur = e.get(&key).map(|v| v.as_str().to_string()).unwrap_or_default();
                ui.horizontal(|ui| {
                    label(ui, &p.label);
                    egui::ComboBox::from_id_salt((uid, &key)).selected_text(&cur).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                        for v in values {
                            if ui.selectable_label(*v == cur, v).clicked() && *v != cur {
                                e.set(&key, Value::Text(v.clone()));
                                changed = true;
                            }
                        }
                    });
                });
            }
            ParamKind::Color { .. } => {
                let mut c = match e.get(&key) {
                    Some(Value::Color(c)) => *c,
                    _ => [1.0; 3],
                };
                ui.horizontal(|ui| {
                    label(ui, &p.label);
                    if ui.color_edit_button_rgb(&mut c).changed() {
                        e.set(&key, Value::Color(c));
                        changed = true;
                    }
                });
            }
            ParamKind::ColorStops { .. } => {
                let mut stops = match e.get(&key) {
                    Some(Value::Stops(s)) => s.clone(),
                    _ => Vec::new(),
                };
                if color_stops_ui(ui, &p.label, &mut stops, (uid, &key)) {
                    e.set(&key, Value::Stops(stops));
                    changed = true;
                }
            }
            ParamKind::Button => {
                ui.horizontal(|ui| {
                    ui.add_space(LABEL_W + 8.0);
                    if ui.button(&p.label).clicked() {
                        if key == "load" {
                            cx.actions.push(Action::PickFile { uid });
                        } else {
                            e.trigger(&key);
                        }
                        changed = true;
                    }
                });
            }
            ParamKind::Text { multiline, .. } => {
                let mut s = e.get(&key).map(|v| v.as_str().to_string()).unwrap_or_default();
                ui.horizontal(|ui| {
                    label(ui, &p.label);
                    let r = if *multiline {
                        ui.add(egui::TextEdit::multiline(&mut s).desired_rows(2).desired_width(f32::INFINITY))
                    } else {
                        ui.add(egui::TextEdit::singleline(&mut s).desired_width(f32::INFINITY))
                    };
                    if r.changed() {
                        e.set(&key, Value::Text(s));
                        changed = true;
                    }
                });
            }
            ParamKind::Info { .. } => {
                let s = e.get(&key).map(|v| v.as_str().to_string()).unwrap_or_default();
                ui.horizontal(|ui| {
                    label(ui, &p.label);
                    ui.label(egui::RichText::new(if s.is_empty() { "–".to_string() } else { s }).weak());
                });
            }
        }
    }
    changed
}

fn color_stops_ui(ui: &mut egui::Ui, title: &str, stops: &mut Vec<ColorStop>, id: (u64, &String)) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        label(ui, title);
        if stops.len() < 6 && ui.small_button("+").on_hover_text("Add color").clicked() {
            let last = stops.last().map(|s| s.stop).unwrap_or(0.0);
            stops.push(ColorStop { color: "#ffffff".into(), stop: (last + 0.2).min(1.0), name: String::new() });
            changed = true;
        }
    });
    let mut remove = None;
    for (i, s) in stops.iter_mut().enumerate() {
        ui.push_id((id.0, id.1, i), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(LABEL_W + 8.0);
                let mut c = phthora_core::params::parse_hex(&s.color);
                if ui.color_edit_button_rgb(&mut c).changed() {
                    s.color = phthora_core::params::to_hex(c);
                    changed = true;
                }
                if ui.add(egui::Slider::new(&mut s.stop, 0.0..=1.0).show_value(false)).changed() {
                    changed = true;
                }
                if ui.small_button("✕").clicked() {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove
        && stops.len() > 2 {
            stops.remove(i);
            changed = true;
        }
    changed
}

const SPEEDS: [f64; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];

fn modulator_ui(ui: &mut egui::Ui, m: &mut Modulator, id: egui::Id, playhead: f64, custom_bands: usize) -> bool {
    let before = m.clone();
    egui::Frame::group(ui.style()).inner_margin(6.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut m.enabled, "on");
            egui::ComboBox::from_id_salt(id.with("shape")).selected_text(m.shape.label()).width(80.0).show_ui(ui, |ui| {
                for s in Shape::ALL {
                    // MIDI input is not available yet
                    if s == Shape::Midi {
                        continue;
                    }
                    ui.selectable_value(&mut m.shape, s, s.label());
                }
            });
            if m.shape == Shape::Audio {
                let bands: Vec<Band> = Band::BUILTIN.into_iter().chain((0..custom_bands as u8).map(Band::Custom)).collect();
                egui::ComboBox::from_id_salt(id.with("band")).selected_text(m.band.name()).width(70.0).show_ui(ui, |ui| {
                    for b in bands {
                        ui.selectable_value(&mut m.band, b, b.name());
                    }
                });
            } else if m.shape != Shape::Keys {
                ui.checkbox(&mut m.looped, "loop").on_hover_text("Speed relative to the effect loop instead of the whole output");
            }
        });
        if m.shape == Shape::Keys {
            if m.keyframes.is_empty() {
                m.keyframes = vec![[0.0, 0.0], [1.0, 1.0]];
            }
            keyframe_editor(ui, id.with("keys"), &mut m.keyframes, playhead);
            ui.label(egui::RichText::new("click: add · drag: move · right click: delete").small().weak());
        } else {
            egui::Grid::new(id.with("grid")).num_columns(4).spacing([6.0, 2.0]).show(ui, |ui| {
                ui.label("speed");
                if m.shape == Shape::Audio {
                    ui.label("–");
                } else if m.looped {
                    egui::ComboBox::from_id_salt(id.with("speed")).selected_text(format!("{}", m.speed)).width(50.0).show_ui(ui, |ui| {
                        for s in SPEEDS {
                            ui.selectable_value(&mut m.speed, s, format!("{s}"));
                        }
                    });
                } else {
                    ui.add(egui::DragValue::new(&mut m.speed).range(0.0..=8.0).speed(0.01));
                }
                ui.label("chance");
                ui.add(egui::DragValue::new(&mut m.chance).range(0.0..=1.0).speed(0.01));
                ui.end_row();
                ui.label("min");
                ui.add_enabled(m.shape != Shape::Const, egui::DragValue::new(&mut m.min).range(0.0..=4.0).speed(0.01));
                ui.label("max");
                ui.add(egui::DragValue::new(&mut m.max).range(0.0..=4.0).speed(0.01));
                ui.end_row();
                ui.label("start");
                ui.add(egui::DragValue::new(&mut m.start).range(0.0..=1.0).speed(0.01));
                ui.label("end");
                ui.add(egui::DragValue::new(&mut m.end).range(0.0..=1.0).speed(0.01));
                ui.end_row();
            });
        }
    });
    *m != before
}

/// Editable piecewise-linear curve over output progress (x) to value (y), both 0..1.
fn keyframe_editor(ui: &mut egui::Ui, id: egui::Id, pts: &mut Vec<[f64; 2]>, playhead: f64) {
    let size = Vec2::new(ui.available_width().max(120.0), 70.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
    let to_screen = |p: [f64; 2]| Pos2::new(rect.left() + p[0] as f32 * rect.width(), rect.bottom() - p[1] as f32 * rect.height());
    let from_screen = |q: Pos2| [((q.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64, ((rect.bottom() - q.y) / rect.height()).clamp(0.0, 1.0) as f64];
    // curve
    let n = 64;
    let line: Vec<Pos2> = (0..=n).map(|i| {
        let x = i as f64 / n as f64;
        to_screen([x, keyframe_value(pts, x)])
    }).collect();
    painter.add(egui::Shape::line(line, Stroke::new(1.5, Color32::from_rgb(120, 220, 160))));
    // playhead
    let px = rect.left() + playhead.clamp(0.0, 1.0) as f32 * rect.width();
    painter.line_segment([Pos2::new(px, rect.top()), Pos2::new(px, rect.bottom())], Stroke::new(1.0, Color32::from_white_alpha(60)));
    let drag_id = id.with("drag");
    let mut dragging: Option<usize> = ui.data(|d| d.get_temp::<Option<usize>>(drag_id)).flatten();
    let mut moved: Option<[f64; 2]> = None;
    let hit = |q: Pos2, pts: &Vec<[f64; 2]>| pts.iter().position(|p| to_screen(*p).distance(q) < 7.0);
    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.drag_started() {
            // egui reports the drag only after a few pixels: hit-test where the press began
            let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(pos);
            dragging = hit(origin, pts);
        }
        if resp.dragged()
            && let Some(i) = dragging
                && i < pts.len() {
                    let q = from_screen(pos);
                    pts[i] = q;
                    moved = Some(q);
                }
        if resp.clicked() && hit(pos, pts).is_none() {
            pts.push(from_screen(pos));
        }
        if resp.secondary_clicked()
            && let Some(i) = hit(pos, pts)
                && pts.len() > 1 {
                    pts.remove(i);
                }
    }
    if resp.drag_stopped() {
        dragging = None;
    }
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    // keep tracking the dragged point after sorting
    if let Some(q) = moved {
        dragging = pts.iter().position(|p| *p == q);
    }
    ui.data_mut(|d| d.insert_temp(drag_id, dragging));
    for p in pts.iter() {
        painter.circle_filled(to_screen(*p), 4.0, Color32::WHITE);
    }
}
