//! Widgets for effect parameters and their modulators.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use phthora_core::modulator::{keyframe_value, Band, Modulator, Shape};
use phthora_core::params::{ColorStop, ParamKind, Value};
use phthora_core::{EffectInstance, OutputMode};
use std::collections::{BTreeMap, HashSet};

pub enum Action {
    /// a "Load..." button: pick a file for the effect's `path`
    PickFile { uid: u64 },
    /// map the next MIDI controller or note to this parameter's modulator
    LearnMidi { uid: u64, key: String },
    /// empty a drawing effect's canvas
    ClearPaint { uid: u64 },
    /// fetch the person segmentation model (Remove Background)
    DownloadModel,
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
    /// modulator waiting for a MIDI message (uid, parameter) and whether it is lit now
    pub learning: Option<(u64, &'a str)>,
    pub blink: bool,
    /// level of the audio track over the output (keyframe editor background)
    pub audio_env: Option<&'a [f32]>,
    /// why the effect cannot do its job (a file that cannot be read)
    pub problem: Option<String>,
    /// fonts in the user's fonts folder
    pub user_fonts: &'a [String],
}

const LABEL_W: f32 = 92.0;

/// Mouse wheel movement this frame in lines (up is positive). Summed from the raw events:
/// the smoothed scroll delta stays zero for some Wayland wheel events.
pub fn wheel_lines(i: &egui::InputState) -> f32 {
    i.events
        .iter()
        .map(|ev| match ev {
            egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta, .. } => delta.y,
            egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta, .. } => delta.y / 40.0,
            egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Page, delta, .. } => delta.y * 10.0,
            _ => 0.0,
        })
        .sum()
}

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
    if e.desc.id == "remove_background" {
        model_status(ui, cx);
    }
    if let Some(p) = &cx.problem {
        ui.colored_label(Color32::from_rgb(240, 110, 100), format!("⚠ {p}"));
    }
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
                            let r = ui.add(egui::Button::new(txt).selected(open).small()).on_hover_text("Modulator (right click: copy, paste)");
                            if r.clicked() {
                                if open {
                                    cx.open_mods.remove(&(uid, key.clone()));
                                } else {
                                    cx.open_mods.insert((uid, key.clone()));
                                }
                            }
                            r.context_menu(|ui| {
                                if let Some(c) = modulator_clipboard(ui, e.modulators.get(&key)) {
                                    let m = e.modulators.entry(key.clone()).or_default();
                                    *m = Modulator { enabled: m.enabled, ..c };
                                    changed = true;
                                }
                            });
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
                        // wheel over the slider: one step per notch (1 % of the range for fine sliders)
                        if r.hovered() && !disabled && !modulated {
                            let lines = ui.input(wheel_lines);
                            if lines != 0.0 {
                                let unit = if *step >= 0.01 * (*max - *min) { step.max(f32::EPSILON) } else { 0.01 * (*max - *min) };
                                v = (v + lines.signum() * lines.abs().ceil() * unit).clamp(*min, *max);
                                if *step > 0.0 {
                                    v = min + ((v - min) / step).round() * step;
                                }
                                changed = true;
                                // the stack must not scroll at the same time
                                ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
                            }
                        }
                        // a slider only senses drags: add clicks, or it never sees a double click
                        if r.interact(egui::Sense::click()).double_clicked() {
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
                    let learn = cx.learning.is_some_and(|(u, k)| u == uid && k == key).then_some(cx.blink);
                    let mut learn_clicked = false;
                    let kx = KeyCtx { title: &p.label, playhead: cx.playhead, env: cx.audio_env };
                    if modulator_ui(ui, m, ui.make_persistent_id((uid, &key)), &kx, cx.custom_bands, learn, &mut learn_clicked) {
                        changed = true;
                    }
                    if learn_clicked {
                        cx.actions.push(Action::LearnMidi { uid, key: key.clone() });
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
            ParamKind::Combo { values, user_fonts, .. } => {
                let cur = e.get(&key).map(|v| v.as_str().to_string()).unwrap_or_default();
                let extra: &[String] = if *user_fonts { cx.user_fonts } else { &[] };
                ui.horizontal(|ui| {
                    label(ui, &p.label);
                    egui::ComboBox::from_id_salt((uid, &key)).selected_text(&cur).width(ui.available_width() - 8.0).height(400.0).show_ui(ui, |ui| {
                        for (i, v) in values.iter().chain(extra).enumerate() {
                            if i == values.len() {
                                ui.separator();
                            }
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
                        } else if key == "clear" && phthora_gpu::paint::PaintKind::of(&e.desc.id).is_some() {
                            cx.actions.push(Action::ClearPaint { uid });
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

/// Remove Background needs a model that is downloaded once: offer it, show the progress.
fn model_status(ui: &mut egui::Ui, cx: &mut ParamCtx) {
    let warn = Color32::from_rgb(230, 190, 90);
    match phthora_seg::status() {
        phthora_seg::Status::Missing => {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(warn, "Needs a person segmentation model, downloaded once.");
                if ui.small_button("Download…").clicked() {
                    cx.actions.push(Action::DownloadModel);
                }
            });
        }
        phthora_seg::Status::Downloading { done, total } => {
            let frac = if total > 0 { done as f32 / total as f32 } else { 0.0 };
            ui.add(egui::ProgressBar::new(frac).text(format!("Downloading model, {} of {} KB", done / 1024, total / 1024)).desired_height(14.0));
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        phthora_seg::Status::Failed(msg) => {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(warn, format!("Model unusable: {msg}"));
                if ui.small_button("Download again…").clicked() {
                    cx.actions.push(Action::DownloadModel);
                }
            });
        }
        phthora_seg::Status::Loading => {
            ui.weak("Loading model…");
        }
        phthora_seg::Status::Available | phthora_seg::Status::Ready => {}
    }
}

/// Copy and paste a modulator between parameters. Returns the modulator to paste: all its
/// settings apply, whether it is on stays as it was.
fn modulator_clipboard(ui: &mut egui::Ui, m: Option<&Modulator>) -> Option<Modulator> {
    let id = egui::Id::unique("modulator clipboard");
    if ui.button("Copy modulator").clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(id, m.cloned().unwrap_or_default()));
        ui.close();
    }
    let clip: Option<Modulator> = ui.ctx().data(|d| d.get_temp(id));
    let paste = ui.add_enabled(clip.is_some(), egui::Button::new("Paste modulator")).clicked();
    if paste {
        ui.close();
    }
    clip.filter(|_| paste)
}

/// `learning`: this modulator waits for MIDI (value: lit in this blink phase).
/// What the keyframe editor shows besides the points.
struct KeyCtx<'a> {
    /// parameter name, for the large editor's title
    title: &'a str,
    playhead: f64,
    env: Option<&'a [f32]>,
}

fn modulator_ui(ui: &mut egui::Ui, m: &mut Modulator, id: egui::Id, kx: &KeyCtx, custom_bands: usize, learning: Option<bool>, learn_clicked: &mut bool) -> bool {
    let before = m.clone();
    egui::Frame::group(ui.style()).inner_margin(6.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut m.enabled, "on");
            egui::ComboBox::from_id_salt(id.with("shape")).selected_text(m.shape.label()).width(80.0).height(400.0).show_ui(ui, |ui| {
                for s in Shape::ALL {
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
            } else if m.shape == Shape::Midi {
                let text = match m.midi_cc {
                    Some(cc) => format!("{cc}"),
                    None => "--".into(),
                };
                let mut t = egui::RichText::new(text).monospace();
                if learning == Some(true) {
                    t = t.color(Color32::BLACK).background_color(Color32::from_rgb(120, 220, 160));
                } else if m.midi_cc.is_some() {
                    t = t.color(Color32::from_rgb(120, 220, 160));
                }
                let hover = if m.midi_device.is_empty() { "MIDI controller or note".to_string() } else { format!("MIDI controller or note on {}", m.midi_device) };
                ui.label(t).on_hover_text(hover);
                let learn = if learning.is_some() { "Waiting…" } else { "Learn" };
                if ui.small_button(learn).on_hover_text("Map the next controller you move (8 s)").clicked() {
                    *learn_clicked = true;
                }
                if ui.add_enabled(m.midi_cc.is_some(), egui::Button::new("Forget").small()).clicked() {
                    m.midi_cc = None;
                    m.midi_device.clear();
                }
            } else if m.shape != Shape::Keys {
                ui.checkbox(&mut m.looped, "loop").on_hover_text("Speed relative to the effect loop instead of the whole output");
            }
        });
        if m.shape == Shape::Keys {
            let big_id = id.with("expanded");
            let mut big = ui.data(|d| d.get_temp::<bool>(big_id)).unwrap_or(false);
            keyframe_editor(ui, id.with("keys"), &mut m.keyframes, kx, 70.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Shift: snap · double click: delete").small().weak())
                    .on_hover_text("Click to add a keyframe, drag to move it (Shift snaps to steps of 0.05), double or right click to delete it. The background shows the loudness of the audio track.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("⛶").on_hover_text("Large editor").clicked() {
                        big = !big;
                    }
                    if ui.add_enabled(!m.keyframes.is_empty(), egui::Button::new("Clear").small()).on_hover_text("Remove all keyframes").clicked() {
                        m.keyframes.clear();
                    }
                });
            });
            if big {
                egui::Window::new(format!("Keyframes: {}", kx.title))
                    .id(id.with("window"))
                    .open(&mut big)
                    .default_size([720.0, 340.0])
                    .show(ui.ctx(), |ui| {
                        let h = (ui.available_height() - 4.0).max(160.0);
                        keyframe_editor(ui, id.with("keys big"), &mut m.keyframes, kx, h);
                    });
            }
            ui.data_mut(|d| d.insert_temp(big_id, big));
        } else if m.shape == Shape::Midi {
            // the controller value maps straight onto the parameter range
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
/// Without points the value stays at 1.
fn keyframe_editor(ui: &mut egui::Ui, id: egui::Id, pts: &mut Vec<[f64; 2]>, kx: &KeyCtx, height: f32) {
    let size = Vec2::new(ui.available_width().max(120.0), height);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, ui.visuals().extreme_bg_color);
    let to_screen = |p: [f64; 2]| Pos2::new(rect.left() + p[0] as f32 * rect.width(), rect.bottom() - p[1] as f32 * rect.height());
    let from_screen = |q: Pos2| [((q.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64, ((rect.bottom() - q.y) / rect.height()).clamp(0.0, 1.0) as f64];
    // audio level over the output, then the grid
    if let Some(env) = kx.env.filter(|e| !e.is_empty()) {
        // scaled to the loudest point: the shape over time is what matters here
        let peak = env.iter().fold(0.0f32, |a, v| a.max(*v));
        if peak > 0.01 {
            let w = rect.width() / env.len() as f32;
            for (i, v) in env.iter().enumerate() {
                let x = rect.left() + i as f32 * w;
                let top = rect.bottom() - (v / peak).clamp(0.0, 1.0) * rect.height() * 0.9;
                painter.rect_filled(Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + w, rect.bottom())), 0.0, Color32::from_rgba_unmultiplied(90, 140, 200, 50));
            }
        }
    }
    let grid = Stroke::new(1.0, Color32::from_white_alpha(14));
    for g in [0.25, 0.5, 0.75] {
        let a = to_screen([g, g]);
        painter.line_segment([Pos2::new(a.x, rect.top()), Pos2::new(a.x, rect.bottom())], grid);
        painter.line_segment([Pos2::new(rect.left(), a.y), Pos2::new(rect.right(), a.y)], grid);
    }
    if pts.is_empty() {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, "click to add keyframes", egui::FontId::proportional(11.0), ui.visuals().weak_text_color());
    } else {
        let n = 128;
        let line: Vec<Pos2> = (0..=n).map(|i| {
            let x = i as f64 / n as f64;
            to_screen([x, keyframe_value(pts, x)])
        }).collect();
        painter.add(egui::Shape::line(line, Stroke::new(1.5, Color32::from_rgb(120, 220, 160))));
    }
    let px = rect.left() + kx.playhead.clamp(0.0, 1.0) as f32 * rect.width();
    painter.line_segment([Pos2::new(px, rect.top()), Pos2::new(px, rect.bottom())], Stroke::new(1.0, Color32::from_white_alpha(60)));

    let drag_id = id.with("drag");
    let mut dragging: Option<usize> = ui.data(|d| d.get_temp::<Option<usize>>(drag_id)).flatten();
    let hit = |q: Pos2, pts: &Vec<[f64; 2]>| pts.iter().position(|p| to_screen(*p).distance(q) < 8.0);
    let snap = ui.input(|i| i.modifiers.shift);
    let place = |q: Pos2| {
        let mut p = from_screen(q);
        if snap {
            p = [(p[0] * 20.0).round() / 20.0, (p[1] * 20.0).round() / 20.0];
        }
        p
    };
    let add = |pts: &mut Vec<[f64; 2]>, p: [f64; 2]| {
        let i = pts.partition_point(|q| q[0] <= p[0]);
        pts.insert(i, p);
        i
    };
    if let Some(pos) = resp.interact_pointer_pos() {
        if resp.drag_started_by(egui::PointerButton::Primary) {
            // egui reports the drag only after a few pixels: hit-test where the press began
            let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(pos);
            dragging = Some(hit(origin, pts).unwrap_or_else(|| add(pts, place(origin))));
        }
        if resp.dragged()
            && let Some(i) = dragging
            && i < pts.len()
        {
            // a point stays between its neighbours, so dragging never reorders
            let lo = if i > 0 { pts[i - 1][0] } else { 0.0 };
            let hi = if i + 1 < pts.len() { pts[i + 1][0] } else { 1.0 };
            let q = place(pos);
            pts[i] = [q[0].clamp(lo, hi), q[1]];
        }
        if resp.double_clicked() || resp.secondary_clicked() {
            if let Some(i) = hit(pos, pts) {
                pts.remove(i);
            }
        } else if resp.clicked() && hit(pos, pts).is_none() {
            add(pts, place(pos));
        }
    }
    if resp.drag_stopped() {
        dragging = None;
    }
    ui.data_mut(|d| d.insert_temp(drag_id, dragging));
    let hovered = resp.hover_pos().and_then(|q| hit(q, pts));
    for (i, p) in pts.iter().enumerate() {
        let active = dragging == Some(i) || hovered == Some(i);
        painter.circle_filled(to_screen(*p), if active { 5.0 } else { 4.0 }, Color32::WHITE);
    }
    if let Some(i) = dragging.or(hovered).filter(|i| *i < pts.len()) {
        let p = pts[i];
        let at = to_screen(p);
        // beside the point, kept inside the editor
        let right = at.x > rect.right() - 80.0;
        let below = at.y < rect.top() + 20.0;
        let align = egui::Align2([if right { egui::Align::Max } else { egui::Align::Min }, if below { egui::Align::Min } else { egui::Align::Max }]);
        let pos = at + Vec2::new(if right { -8.0 } else { 8.0 }, if below { 6.0 } else { -6.0 });
        painter.text(pos, align, format!("{:.3}, {:.3}", p[0], p[1]), egui::FontId::monospace(10.0), Color32::WHITE);
    }
}
