//! Main window.

use crate::audio_ui::{AudioState, Src};
use phthora_audio::{MidiIn, MidiValues};
use phthora_gpu::paint::{self, PaintCanvas, PaintKind};
use phthora_core::orbit::{self, Orbit};
use phthora_gpu::Readback;
use phthora_media::{LiveSource, Record, VideoDevice, VirtualCam};
use crate::params_ui::{self, Action, ParamCtx};
use crate::pipeline::{InputVideo, Preview, Step};
use crate::render::{self, ExportJob, Progress};
use crate::store::{self, Settings};
use eframe::egui::{self, Color32, Key, KeyboardShortcut, Modifiers, Pos2, Rect, Sense, Vec2};
use eframe::egui_wgpu;
use phthora_core::clock::BpmTap;
use phthora_core::history::History;
use phthora_core::params::Value;
use phthora_core::registry::{self, categories};
use phthora_core::{Chain, FrameTime, OutputMode, Playback, ResolvedEffect, TimingSettings};
use phthora_gpu::{Engine, GpuContext, Target};
use phthora_media::{Backend, Format, MediaInfo, MediaKind, Quality};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Instant;

const MAX_DIM: u32 = 4096;
const DEFAULT_SIZE: (u32, u32) = (1280, 720);

enum Source {
    None,
    Still,
    Video(InputVideo),
    /// webcam; the effect clock runs in 4 s loops on real time, like the reference
    Live(LiveSource),
}

/// Loop length of the live clock (webcam), seconds.
const LIVE_LOOP: f64 = 4.0;
/// Size the webcam is shown at.
const LIVE_SIZE: (u32, u32) = (1280, 720);

/// A running export: worker thread, progress, start time.
type RunningExport = (std::thread::JoinHandle<Result<PathBuf, String>>, Progress, Instant);

/// MIDI learn in progress: the parameter, the message count when it began, the start time.
struct Learn {
    uid: u64,
    key: String,
    since: u64,
    started: Instant,
}

/// A brush stroke in progress, in canvas pixels.
struct Stroke {
    uid: u64,
    /// where the last brush step was placed
    last: Pos2,
    /// brush steps owed since the last frame (fractional)
    carry: f64,
    at: Instant,
}

/// How long MIDI learn waits for a message.
const LEARN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

struct ExportUi {
    open: bool,
    format: Format,
    quality: Quality,
    running: Option<RunningExport>,
}

pub struct PhthoraApp {
    engine: Engine,
    render_state: egui_wgpu::RenderState,
    display: Option<(Target, egui::TextureId)>,
    chain: Chain,
    history: History,
    preview: Preview,
    midi: MidiIn,
    midi_values: MidiValues,
    /// modulator waiting for the next MIDI message
    learning: Option<Learn>,
    /// drawing effect chosen to receive mouse strokes (else the last one in the stack)
    draw_target: Option<u64>,
    stroke: Option<Stroke>,
    /// clock of the live input (webcam)
    live_clock: Instant,
    /// recording started at (webcam)
    rec_started: Option<Instant>,
    /// video devices, refreshed when the settings open
    devices: Vec<VideoDevice>,
    /// preview frames to a virtual camera, with the readback that feeds it
    vcam: Option<(VirtualCam, Readback)>,
    playback: Playback,
    /// VAAPI H.264 works here (checked in the background at start)
    vaapi: std::sync::Arc<std::sync::atomic::AtomicBool>,
    timing: TimingSettings,
    bpm: f64,
    use_bpm: bool,
    bpm_tap: BpmTap,
    source: Source,
    input_path: Option<PathBuf>,
    input_info: Option<MediaInfo>,
    out_size: (u32, u32),
    keep_aspect: bool,
    playing: bool,
    play_time: f64,
    last_tick: Option<Instant>,
    presets: Vec<PathBuf>,
    preset_idx: Option<usize>,
    preset_name: String,
    preset_scan: Instant,
    export: ExportUi,
    audio: AudioState,
    settings: Settings,
    show_settings: bool,
    status: String,
    status_time: Option<Instant>,
    renaming: Option<(u64, String)>,
    open_mods: HashSet<(u64, String)>,
    add_filter: String,
    live: HashMap<u64, BTreeMap<String, Value>>,
    last_ft: FrameTime,
    /// the preview waits for the decoder: repaint soon
    waiting: bool,
}

impl PhthoraApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Self {
        let rs = cc.wgpu_render_state.clone().expect("wgpu renderer");
        let ctx = GpuContext::from_parts(rs.device.clone(), rs.queue.clone());
        let settings = Settings::load();
        let mut engine = Engine::new(&ctx, DEFAULT_SIZE.0, DEFAULT_SIZE.1);
        crate::video::install(&mut engine.assets, true);
        engine.background = settings.background;
        engine.flip_x = settings.flip_x;
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_gray(22);
        cc.egui_ctx.set_visuals(visuals);
        // labels are UI chrome, not text to copy: dragging stack rows must not select names
        cc.egui_ctx.all_styles_mut(|s| s.interaction.selectable_labels = false);
        let quality = parse_quality(&settings.quality);
        let mut app = Self {
            engine,
            render_state: rs,
            display: None,
            chain: Chain::default(),
            history: History::default(),
            preview: Preview::default(),
            midi: MidiIn::start(),
            midi_values: MidiValues::new(),
            learning: None,
            draw_target: None,
            stroke: None,
            live_clock: Instant::now(),
            rec_started: None,
            devices: Vec::new(),
            vcam: None,
            playback: Playback::default(),
            vaapi: Default::default(),
            timing: TimingSettings::default(),
            bpm: 120.0,
            use_bpm: false,
            bpm_tap: BpmTap::default(),
            source: Source::None,
            input_path: None,
            input_info: None,
            out_size: DEFAULT_SIZE,
            keep_aspect: true,
            playing: true,
            play_time: 0.0,
            last_tick: None,
            presets: store::list_presets(),
            preset_idx: None,
            preset_name: String::new(),
            preset_scan: Instant::now(),
            export: ExportUi { open: false, format: Format::Mp4, quality, running: None },
            audio: AudioState::new(settings.audio.clone()),
            settings,
            show_settings: false,
            status: String::new(),
            status_time: None,
            renaming: None,
            open_mods: HashSet::new(),
            add_filter: String::new(),
            live: HashMap::new(),
            last_ft: FrameTime::default(),
            waiting: false,
        };
        if let Some(p) = initial {
            app.load_input(&p);
        }
        let flag = app.vaapi.clone();
        std::thread::spawn(move || flag.store(phthora_media::vaapi_available(), Ordering::Relaxed));
        app
    }

    /// The drawing effect that receives mouse strokes.
    fn active_draw(&self) -> Option<u64> {
        let drawable = |e: &&phthora_core::EffectInstance| e.enabled && PaintKind::of(&e.desc.id).is_some();
        let mut fx = self.chain.effects.iter().filter(drawable);
        match self.draw_target {
            Some(u) if self.chain.effects.iter().filter(drawable).any(|e| e.uid == u) => Some(u),
            _ => fx.next_back().map(|e| e.uid),
        }
    }

    /// Keep the engine's canvases in step with the drawing effects (size, undo, presets).
    fn sync_paint(&mut self) {
        let size = self.engine.size();
        let mut alive = HashSet::new();
        for e in &self.chain.effects {
            let Some(kind) = PaintKind::of(&e.desc.id) else { continue };
            alive.insert(e.uid);
            let c = self.engine.paint.entry(e.uid).or_insert_with(|| PaintCanvas::new(kind, size.0, size.1));
            c.resize(size.0, size.1);
            let url = e.data.get(paint::DATA_KEY).map(String::as_str).unwrap_or("");
            // a stroke in progress is newer than the stored data
            if url != c.synced && self.stroke.as_ref().is_none_or(|s| s.uid != e.uid) {
                c.load_data_url(url);
            }
        }
        self.engine.paint.retain(|u, _| alive.contains(u));
        // turning mouse control off resets the camera
        for e in &mut self.chain.effects {
            if e.data.contains_key(orbit::DATA_KEY) && !e.get("mouseControls").is_some_and(|v| v.as_bool()) {
                e.data.remove(orbit::DATA_KEY);
            }
        }
    }

    /// 3D effects whose camera follows the mouse.
    fn orbit_targets(&self) -> Vec<u64> {
        self.chain.effects.iter().filter(|e| e.enabled && e.get("mouseControls").is_some_and(|v| v.as_bool())).map(|e| e.uid).collect()
    }

    /// Mouse camera on the preview: drag turns, right drag (or Shift+drag) pans, wheel zooms.
    fn orbit_input(&mut self, ui: &egui::Ui, resp: &egui::Response, rect: Rect, targets: &[u64]) {
        let d = resp.drag_delta();
        let shift = ui.input(|i| i.modifiers.shift);
        let pan = resp.dragged_by(egui::PointerButton::Secondary) || (shift && resp.dragged_by(egui::PointerButton::Primary));
        let turn = !pan && resp.dragged_by(egui::PointerButton::Primary);
        // wheel notches (a notch is a line, about 40 points of touchpad scrolling): 5 % each
        let wheel = if resp.hovered() {
            ui.input(|i| {
                i.events
                    .iter()
                    .map(|ev| match ev {
                        egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta, .. } => delta.y,
                        egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta, .. } => delta.y / 40.0,
                        egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Page, delta, .. } => delta.y * 10.0,
                        _ => 0.0,
                    })
                    .sum::<f32>()
            })
        } else {
            0.0
        };
        if d == Vec2::ZERO && wheel == 0.0 {
            return;
        }
        for uid in targets {
            let Some(e) = self.chain.get_mut(*uid) else { continue };
            let mut o = e.data.get(orbit::DATA_KEY).and_then(|s| Orbit::parse(s)).unwrap_or_default();
            if turn {
                o.rotate(d.x, d.y, rect.height());
            } else if pan {
                o.pan(d.x, d.y, rect.height());
            }
            o.zoom(wheel);
            e.data.insert(orbit::DATA_KEY.into(), o.to_data());
        }
    }

    /// Brush input on the preview at `rect` (the displayed output).
    fn draw_input(&mut self, ui: &egui::Ui, resp: &egui::Response, rect: Rect) {
        let Some(uid) = self.active_draw() else {
            self.stroke = None;
            return;
        };
        let down = resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_down());
        let Some(c) = self.engine.paint.get_mut(&uid) else { return };
        let (cw, ch) = c.size();
        let to_canvas = |p: Pos2| Pos2::new((p.x - rect.left()) / rect.width() * cw as f32, (p.y - rect.top()) / rect.height() * ch as f32);
        let pos = ui.input(|i| i.pointer.latest_pos()).map(to_canvas);
        match (&mut self.stroke, down, pos) {
            (None, true, Some(p)) if rect.contains(resp.interact_pointer_pos().unwrap_or(Pos2::ZERO)) => {
                // the first step lands right away, without a jump from an old position
                self.stroke = Some(Stroke { uid, last: p, carry: 1.0, at: Instant::now() });
            }
            (Some(s), true, Some(p)) if s.uid == uid => {
                let now = Instant::now();
                s.carry += now.duration_since(s.at).as_secs_f64() * paint::DAB_RATE;
                s.at = now;
                let n = s.carry.floor() as u32;
                s.carry -= n as f64;
                let erase = self.chain.effects.iter().find(|e| e.uid == uid).is_some_and(|e| e.get("erase").is_some_and(|v| v.as_bool()));
                let from = s.last;
                for k in 1..=n {
                    let q = from + (p - from) * (k as f32 / n as f32);
                    c.dab(q.x, q.y, (q.x - s.last.x, q.y - s.last.y), erase);
                    s.last = q;
                }
                ui.ctx().request_repaint();
            }
            (Some(_), false, _) | (Some(_), _, None) => self.end_stroke(),
            _ => {}
        }
    }

    /// Store the finished stroke in the effect, so presets and undo see it.
    fn end_stroke(&mut self) {
        let Some(s) = self.stroke.take() else { return };
        if let (Some(c), Some(e)) = (self.engine.paint.get_mut(&s.uid), self.chain.get_mut(s.uid)) {
            let url = c.to_data_url();
            c.synced = url.clone();
            e.data.insert(paint::DATA_KEY.into(), url);
        }
    }

    /// Take the newest MIDI values and finish a pending learn.
    fn poll_midi(&mut self) {
        self.midi_values = self.midi.values();
        let Some(l) = &self.learning else { return };
        if let Some((dev, num)) = self.midi.last_since(l.since) {
            if let Some(m) = self.chain.get_mut(l.uid).and_then(|e| e.modulators.get_mut(&l.key)) {
                m.midi_cc = Some(num);
                m.midi_device = dev.clone();
            }
            self.learning = None;
            self.notify(format!("MIDI {num} on {dev} mapped"));
        } else if l.started.elapsed() > LEARN_TIMEOUT {
            self.learning = None;
            self.notify("MIDI learn: nothing received");
        }
    }

    fn notify(&mut self, s: impl Into<String>) {
        self.status = s.into();
        self.status_time = Some(Instant::now());
    }

    // ------------------------------------------------------------ input

    fn load_input(&mut self, path: &Path) {
        let info = match phthora_media::probe(path) {
            Ok(i) => i,
            Err(e) => {
                self.notify(format!("Cannot open {}: {e}", path.display()));
                return;
            }
        };
        let (w, h) = phthora_media::fit_size(info.width, info.height, MAX_DIM);
        self.set_size(w, h);
        if info.kind == MediaKind::Image {
            match phthora_media::load_image(path, MAX_DIM) {
                Ok(img) => {
                    self.engine.set_source_rgba(img.width, img.height, &img.data);
                    self.source = Source::Still;
                    self.timing.mode = OutputMode::Image;
                    self.export.format = Format::Png;
                }
                Err(e) => {
                    self.notify(format!("Cannot load image: {e}"));
                    return;
                }
            }
        } else {
            self.playback = Playback { speed: self.playback.speed, direction: self.playback.direction, ..Default::default() };
            let v = InputVideo::open(path, &info, (w, h), self.playback);
            self.timing.mode = OutputMode::Video;
            self.timing.duration = v.playback.natural_duration(v.clip()).clamp(0.05, 3600.0);
            if info.fps > 0.0 {
                self.timing.fps = info.fps.round().clamp(1.0, 120.0);
            }
            if self.export.format == Format::Png {
                self.export.format = Format::Mp4;
            }
            self.source = Source::Video(v);
        }
        self.settings.input_dir = path.parent().map(|p| p.to_path_buf());
        self.settings.save();
        if self.audio.src == Src::None && info.has_audio {
            self.audio.src = Src::Input;
        }
        self.input_info = Some(info);
        self.input_path = Some(path.to_path_buf());
        self.play_time = 0.0;
        self.engine.reset_history();
        self.preview.invalidate();
        self.notify(format!("Loaded {}", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    }

    fn set_size(&mut self, w: u32, h: u32) {
        let (w, h) = (w.clamp(16, MAX_DIM) & !1, h.clamp(16, MAX_DIM) & !1);
        self.out_size = (w, h);
        self.engine.set_size(w, h);
        self.reopen_video();
        if let Source::Live(cam) = &self.source
            && cam.size() != (w, h) && self.rec_started.is_none() {
                self.start_webcam(None);
            }
    }

    /// Restart the input video with the current size and playback settings.
    fn reopen_video(&mut self) {
        if let Source::Video(v) = &mut self.source
            && let Some(path) = self.input_path.clone() {
                let info = v.reader.info().clone();
                *v = InputVideo::open(&path, &info, self.out_size, self.playback);
            }
        self.preview.invalidate();
    }

    fn pick_input(&mut self) {
        let mut d = rfd::FileDialog::new().add_filter("Media", &["mp4", "mov", "webm", "ogg", "avi", "mkv", "m4v", "gif", "png", "jpg", "jpeg", "webp"]);
        if let Some(dir) = &self.settings.input_dir {
            d = d.set_directory(dir);
        }
        if let Some(p) = d.pick_file() {
            self.load_input(&p);
        }
    }

    // ------------------------------------------------------------ presets

    fn rescan_presets(&mut self) {
        let cur = self.preset_idx.and_then(|i| self.presets.get(i).cloned());
        self.presets = store::list_presets();
        self.preset_idx = cur.and_then(|c| self.presets.iter().position(|p| *p == c));
    }

    fn load_preset(&mut self, idx: usize) {
        let Some(path) = self.presets.get(idx).cloned() else { return };
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| phthora_core::preset::from_json(&s).map_err(|e| e.to_string())) {
            Ok((effects, warnings)) => {
                self.chain.replace(effects, true);
                self.history.observe(&self.chain.effects);
                self.preset_idx = Some(idx);
                self.preset_name = store::preset_label(&path);
                self.engine.reset_history();
                if warnings.is_empty() {
                    self.notify(format!("Preset: {}", self.preset_name));
                } else {
                    self.notify(format!("Preset {} loaded with warnings: {}", self.preset_name, warnings.join("; ")));
                }
            }
            Err(e) => self.notify(format!("Cannot load preset: {e}")),
        }
    }

    fn step_preset(&mut self, d: i64) {
        if self.presets.is_empty() {
            return;
        }
        let n = self.presets.len() as i64;
        let i = match self.preset_idx {
            Some(i) => (i as i64 + d).rem_euclid(n),
            None => if d >= 0 { 0 } else { n - 1 },
        };
        self.load_preset(i as usize);
    }

    fn random_preset(&mut self) {
        if self.presets.is_empty() {
            return;
        }
        use rand::Rng;
        let mut i = rand::rng().random_range(0..self.presets.len());
        if self.presets.len() > 1 && Some(i) == self.preset_idx {
            i = (i + 1) % self.presets.len();
        }
        self.load_preset(i);
    }

    fn save_preset(&mut self, name: &str) {
        let name = store::sanitize_name(name);
        if name.is_empty() {
            self.notify("Preset name: letters, digits, space, - and _ only");
            return;
        }
        let path = store::preset_path(&name);
        let json = phthora_core::preset::to_json(&self.chain.effects);
        match std::fs::write(&path, json) {
            Ok(()) => {
                self.rescan_presets();
                self.preset_idx = self.presets.iter().position(|p| *p == path);
                self.preset_name = name.clone();
                self.notify(format!("Saved preset {name}"));
            }
            Err(e) => self.notify(format!("Cannot save preset: {e}")),
        }
    }

    fn update_preset(&mut self) {
        let Some(path) = self.preset_idx.and_then(|i| self.presets.get(i).cloned()) else {
            self.notify("No preset selected");
            return;
        };
        match std::fs::write(&path, phthora_core::preset::to_json(&self.chain.effects)) {
            Ok(()) => self.notify(format!("Updated preset {}", store::preset_label(&path))),
            Err(e) => self.notify(format!("Cannot save preset: {e}")),
        }
    }

    fn delete_preset(&mut self) {
        let Some(path) = self.preset_idx.and_then(|i| self.presets.get(i).cloned()) else { return };
        if std::fs::remove_file(&path).is_ok() {
            self.notify(format!("Deleted preset {}", store::preset_label(&path)));
            self.preset_idx = None;
            self.preset_name.clear();
            self.rescan_presets();
        }
    }

    // ------------------------------------------------------------ export

    /// The input has audio that still lines up with the picture (not reversed or re-timed).
    fn input_audio_usable(&self) -> bool {
        self.input_info.as_ref().is_some_and(|i| i.has_audio) && self.playback.speed == 1.0 && self.playback.direction == phthora_core::Direction::Forward
    }

    fn start_export(&mut self) {
        if self.export.running.is_some() {
            return;
        }
        let fmt = self.export.format;
        let stem = self.input_path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "phthora".into());
        let name = phthora_media::default_export_name(&stem, fmt.extension());
        let Some(out) = rfd::FileDialog::new().set_directory(self.settings.export_dir()).set_file_name(&name).save_file() else { return };
        self.settings.output_dir = out.parent().map(|p| p.to_path_buf());
        self.settings.quality = quality_name(self.export.quality).into();
        self.settings.save();
        let audio = self.audio.export_source();
        self.settings.audio = self.audio.settings.clone();
        let mut timing = self.timing;
        timing.bpm = self.use_bpm.then_some(self.bpm);
        let job = ExportJob {
            input: self.input_path.clone(),
            playback: self.playback,
            effects: self.chain.effects.clone(),
            timing,
            width: self.out_size.0,
            height: self.out_size.1,
            background: self.engine.background,
            flip_x: self.engine.flip_x,
            format: fmt,
            quality: self.export.quality,
            backend: if self.settings.hw_encode && self.vaapi.load(Ordering::Relaxed) { Backend::Vaapi } else { Backend::Software },
            audio,
            audio_track: self.audio.track.clone().filter(|_| self.audio.src != Src::Mic),
            audio_settings: self.audio.settings.clone(),
            midi: self.midi_values.clone(),
            out,
        };
        let ctx = self.engine.ctx.clone();
        let progress = Progress::default();
        let p2 = progress.clone();
        let handle = std::thread::spawn(move || render::run(&ctx, &job, &p2));
        self.export.running = Some((handle, progress, Instant::now()));
    }

    fn poll_export(&mut self) {
        let finished = self.export.running.as_ref().map(|(h, _, _)| h.is_finished()).unwrap_or(false);
        if finished {
            let (h, _, t0) = self.export.running.take().unwrap();
            match h.join() {
                Ok(Ok(p)) => self.notify(format!("Exported {} in {:.1} s", p.display(), t0.elapsed().as_secs_f32())),
                Ok(Err(e)) if e == "cancelled" => self.notify("Export cancelled"),
                Ok(Err(e)) => self.notify(format!("Export failed: {}", e.lines().last().unwrap_or(&e))),
                Err(_) => self.notify("Export crashed"),
            }
        }
    }

    // ------------------------------------------------------------ frame

    fn timing_now(&self) -> TimingSettings {
        let mut t = self.timing;
        t.bpm = self.use_bpm.then_some(self.bpm);
        t
    }

    /// Output frame index at the playback position.
    fn frame_index(&self) -> u64 {
        let t = self.timing_now();
        match t.mode {
            OutputMode::Image => 0,
            OutputMode::Video => ((self.play_time * t.fps + 1e-6).floor() as u64) % t.frame_count(),
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let input_audio = self.input_audio_usable();
        self.audio.sync(self.input_path.as_deref(), input_audio, (self.playback.trim_start, self.playback.trim_end));
        let video_playing = self.playing && self.timing.mode == OutputMode::Video;
        if video_playing
            && let Some(last) = self.last_tick {
                self.play_time += now.duration_since(last).as_secs_f64();
                if self.play_time >= self.timing.duration {
                    self.play_time %= self.timing.duration.max(0.01);
                }
            }
        self.audio.play(video_playing, self.play_time, self.timing.duration);
        self.last_tick = Some(now);
        let alive: HashSet<u64> = self.chain.effects.iter().map(|e| e.uid).collect();
        self.engine.retain(|u| alive.contains(&u));
        if matches!(self.source, Source::Live(_)) {
            self.tick_live();
            return;
        }
        let timing = self.timing_now();
        let n = self.frame_index();
        let ft = timing.at_frame(n);
        self.last_ft = ft;
        let live = self.audio.inputs(n, timing.fps);
        self.engine.audio_spectrum.clone_from(&self.audio.spectrum);
        self.poll_midi();
        self.sync_paint();
        let resolved: Vec<ResolvedEffect> = self.chain.resolve(&ft, &live, &self.midi_values);
        self.live = resolved.iter().map(|r| (r.uid, r.values.clone())).collect();
        let video = match &self.source {
            Source::Video(v) => Some(v),
            _ => None,
        };
        match self.preview.step(&mut self.engine, video, n, &timing, resolved) {
            Step::Rendered => self.on_rendered(),
            Step::Waiting => self.waiting = true,
            Step::Unchanged => {}
        }
    }

    /// Webcam frame: effects run on the live clock (4 s loops of real time).
    fn tick_live(&mut self) {
        let Source::Live(cam) = &self.source else { return };
        if let Some(e) = cam.error() {
            self.stop_webcam();
            self.notify(format!("Webcam stopped: {}", e.lines().last().unwrap_or(&e)));
            return;
        }
        let Some((seq, frame)) = cam.latest() else {
            self.waiting = true;
            return;
        };
        let timing = TimingSettings { mode: OutputMode::Video, duration: LIVE_LOOP, fps: 30.0, bpm: self.use_bpm.then_some(self.bpm) };
        let mut ft = timing.at_seconds(self.live_clock.elapsed().as_secs_f64());
        ft.frame = seq;
        self.last_ft = ft;
        let mut live = self.audio.inputs(seq, timing.fps);
        // start and end of modulators mean nothing without an output length
        live.live = true;
        self.engine.audio_spectrum.clone_from(&self.audio.spectrum);
        self.poll_midi();
        self.sync_paint();
        let resolved: Vec<ResolvedEffect> = self.chain.resolve(&ft, &live, &self.midi_values);
        self.live = resolved.iter().map(|r| (r.uid, r.values.clone())).collect();
        if self.preview.step_live(&mut self.engine, seq, &frame, &ft, resolved) == Step::Rendered {
            self.on_rendered();
        }
        // poll for the next camera frame
        self.waiting = true;
    }

    fn on_rendered(&mut self) {
        self.update_display();
        self.feed_virtual_cam();
    }

    /// Send the newest preview frame to the virtual camera (two frames behind: readback ring).
    fn feed_virtual_cam(&mut self) {
        let size = self.engine.size();
        if self.vcam.as_ref().is_some_and(|(v, _)| v.size() != size || v.error().is_some()) {
            if let Some(e) = self.vcam.as_ref().and_then(|(v, _)| v.error()) {
                self.notify(format!("Virtual camera: {}", e.lines().last().unwrap_or(&e)));
                self.settings.virtual_cam = None;
            }
            self.vcam = None;
        }
        if self.vcam.is_none()
            && let Some(dev) = self.settings.virtual_cam.clone() {
                match VirtualCam::open(&dev, size.0, size.1, 30.0) {
                    Ok(v) => self.vcam = Some((v, Readback::new(&self.engine.ctx, &self.engine.lib, size.0, size.1))),
                    Err(e) => {
                        self.notify(format!("Virtual camera: {e}"));
                        self.settings.virtual_cam = None;
                    }
                }
            }
        if let Some((v, rb)) = &mut self.vcam
            && let Some(px) = rb.push(self.engine.output(), self.engine.blank()) {
                v.push(px);
            }
    }

    /// Camera devices first, then loopback devices that currently carry video.
    fn cameras(&self) -> Vec<VideoDevice> {
        let mut v: Vec<VideoDevice> = self.devices.iter().filter(|d| d.capture && Some(&d.path) != self.settings.virtual_cam.as_ref()).cloned().collect();
        v.sort_by_key(|d| d.loopback);
        v
    }

    fn start_webcam(&mut self, record: Option<Record>) {
        if self.devices.is_empty() {
            self.devices = phthora_media::list_devices();
        }
        let cams = self.cameras();
        let dev = match &self.settings.camera {
            Some(p) if cams.iter().any(|d| &d.path == p) => p.clone(),
            _ => match cams.first() {
                Some(d) => d.path.clone(),
                None => {
                    self.notify("No webcam found");
                    return;
                }
            },
        };
        // the device is busy while the old capture runs
        let was_live = matches!(self.source, Source::Live(_));
        if was_live {
            self.source = Source::None;
        }
        if !was_live && self.out_size != LIVE_SIZE {
            self.set_size(LIVE_SIZE.0, LIVE_SIZE.1);
        }
        let (w, h) = self.out_size;
        match LiveSource::open(&dev, w, h, record.clone()) {
            Ok(cam) => {
                self.source = Source::Live(cam);
                self.settings.camera = Some(dev);
                self.input_path = None;
                self.input_info = None;
                self.timing.mode = OutputMode::Video;
                self.engine.flip_x = self.settings.flip_webcam;
                self.engine.reset_history();
                self.preview.invalidate();
                self.rec_started = record.is_some().then(Instant::now);
                if record.is_none() {
                    self.notify("Webcam on");
                }
            }
            Err(e) => self.notify(format!("Cannot start the webcam: {e}")),
        }
    }

    fn stop_webcam(&mut self) {
        if let Source::Live(cam) = std::mem::replace(&mut self.source, Source::None) {
            self.settings.flip_webcam = self.engine.flip_x;
            self.engine.flip_x = self.settings.flip_x;
            self.rec_started = None;
            drop(cam);
            self.preview.invalidate();
        }
    }

    /// Start or finish a webcam recording. A finished recording becomes the input and the
    /// export opens, so the effects can be rendered onto it.
    fn toggle_recording(&mut self) {
        let Source::Live(_) = self.source else { return };
        if self.rec_started.is_none() {
            let dir = store::data_dir().join("recordings");
            let _ = std::fs::create_dir_all(&dir);
            let path = dir.join(phthora_media::default_export_name("webcam", "mkv"));
            // the microphone is recorded when it is the audio source
            let audio = self.audio.src == Src::Mic;
            self.start_webcam(Some(Record { path, audio }));
            self.notify(if audio { "Recording webcam and microphone" } else { "Recording webcam" });
            return;
        }
        let Source::Live(cam) = std::mem::replace(&mut self.source, Source::None) else { return };
        self.settings.flip_webcam = self.engine.flip_x;
        self.rec_started = None;
        match cam.stop() {
            Some(path) => {
                self.load_input(&path);
                // the recording shows the camera as it was seen
                self.engine.flip_x = self.settings.flip_webcam;
                if self.audio.src == Src::Mic {
                    self.audio.src = Src::Input;
                }
                self.export.open = true;
            }
            None => {
                self.engine.flip_x = self.settings.flip_x;
                self.notify("The recording could not be written");
            }
        }
    }

    fn update_display(&mut self) {
        let (w, h) = self.engine.size();
        let need_new = self.display.as_ref().map(|(t, _)| t.size() != (w, h)).unwrap_or(true);
        if need_new {
            let t = Target::new(&self.engine.ctx, w, h, "display");
            let mut r = self.render_state.renderer.write();
            let id = match self.display.take() {
                Some((_, id)) => {
                    r.update_egui_texture_from_wgpu_texture(&self.render_state.device, &t.view, wgpu::FilterMode::Linear, id);
                    id
                }
                None => r.register_native_texture(&self.render_state.device, &t.view, wgpu::FilterMode::Linear),
            };
            drop(r);
            self.display = Some((t, id));
        }
        let (disp, _) = self.display.as_ref().unwrap();
        let out = self.engine.output();
        let mut enc = self.engine.ctx.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo { texture: &out.texture, mip_level: 0, origin: Default::default(), aspect: Default::default() },
            wgpu::TexelCopyTextureInfo { texture: &disp.texture, mip_level: 0, origin: Default::default(), aspect: Default::default() },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.engine.ctx.queue.submit([enc.finish()]);
    }

    // ------------------------------------------------------------ UI pieces

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = |k| KeyboardShortcut::new(Modifiers::COMMAND, k);
        let typing = ctx.egui_wants_keyboard_input();
        if !typing && ctx.input(|i| i.key_pressed(Key::Space)) {
            self.playing = !self.playing;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::O))) {
            self.pick_input();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::W))) {
            if matches!(self.source, Source::Live(_)) { self.stop_webcam() } else { self.start_webcam(None) }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::S))) {
            self.export.open = true;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::M))) {
            self.chaos();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z))) {
            self.redo();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::Z))) {
            self.undo();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::D))) {
            self.chain.disable_all();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::E))) {
            self.chain.shuffle();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::ArrowRight))) {
            self.step_preset(1);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::ArrowLeft))) {
            self.step_preset(-1);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::R))) {
            self.random_preset();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&cmd(Key::U))) {
            self.update_preset();
        }
        let digits = [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];
        for (i, k) in digits.into_iter().enumerate() {
            if ctx.input_mut(|inp| inp.consume_shortcut(&cmd(k))) {
                // Ctrl+1..9 load the first nine presets, Ctrl+0 the tenth
                self.load_preset((i + 9) % 10);
            }
        }
    }

    fn chaos(&mut self) {
        self.history.observe(&self.chain.effects);
        self.chain.chaos();
        self.history.observe(&self.chain.effects);
        self.engine.reset_history();
    }

    fn undo(&mut self) {
        if let Some(v) = self.history.undo(&self.chain.effects) {
            self.chain.effects = v;
        }
    }

    fn redo(&mut self) {
        if let Some(v) = self.history.redo(&self.chain.effects) {
            self.chain.effects = v;
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Open…").on_hover_text("Ctrl+O").clicked() {
                self.pick_input();
            }
            let live = matches!(self.source, Source::Live(_));
            if ui.add(egui::Button::new("Webcam").selected(live)).on_hover_text("Ctrl+W").clicked() {
                if live { self.stop_webcam() } else { self.start_webcam(None) }
            }
            if live {
                let txt = match self.rec_started {
                    Some(t) => egui::RichText::new(format!("■ {:.0} s", t.elapsed().as_secs_f64().floor())).color(Color32::from_rgb(240, 90, 90)),
                    None => egui::RichText::new("● Rec").color(Color32::from_rgb(240, 90, 90)),
                };
                if ui.button(txt).on_hover_text("Record the webcam, then export it with the effects").clicked() {
                    self.toggle_recording();
                }
            }
            ui.separator();
            let mut mode = self.timing.mode;
            // the webcam always runs as video
            ui.add_enabled_ui(!live, |ui| {
                ui.selectable_value(&mut mode, OutputMode::Image, "Image");
                ui.selectable_value(&mut mode, OutputMode::Video, "Video");
            });
            if mode != self.timing.mode {
                self.timing.mode = mode;
                self.export.format = if mode == OutputMode::Image { Format::Png } else if self.export.format == Format::Png { Format::Mp4 } else { self.export.format };
                self.engine.reset_history();
                self.preview.invalidate();
            }
            ui.separator();
            if self.timing.mode == OutputMode::Video && !matches!(self.source, Source::Live(_)) {
                if ui.button(if self.playing { "⏸" } else { "▶" }).on_hover_text("Space").clicked() {
                    self.playing = !self.playing;
                }
                let dur = self.timing.duration;
                let mut t = self.play_time;
                ui.spacing_mut().slider_width = 220.0;
                if ui.add(egui::Slider::new(&mut t, 0.0..=dur).show_value(false)).changed() {
                    self.play_time = t;
                }
                ui.label(format!("{:5.2} / {:.2} s", self.play_time, dur));
                ui.separator();
            }
            if ui.button("Chaos").on_hover_text("Random effects (Ctrl+M)").clicked() {
                self.chaos();
            }
            if ui.add_enabled(self.history.can_undo(), egui::Button::new("Undo")).on_hover_text("Ctrl+Z").clicked() {
                self.undo();
            }
            if ui.add_enabled(self.history.can_redo(), egui::Button::new("Redo")).on_hover_text("Ctrl+Shift+Z").clicked() {
                self.redo();
            }
            if ui.button("Shuffle").on_hover_text("Shuffle effect order (Ctrl+E)").clicked() {
                self.chain.shuffle();
            }
            if ui.button("Clear").on_hover_text("Remove all unlocked effects").clicked() {
                self.chain.clear_unlocked();
            }
            ui.separator();
            if ui.button("Export…").on_hover_text("Ctrl+S").clicked() {
                self.export.open = true;
            }
            if ui.button("Audio").on_hover_text("Audio source, gain and bands").clicked() {
                self.audio.open = !self.audio.open;
            }
            let mute = if self.audio.muted { "🔇" } else { "🔊" };
            if ui.button(mute).on_hover_text("Mute the preview").clicked() {
                self.audio.muted = !self.audio.muted;
            }
            if ui.button("⚙").on_hover_text("Settings").clicked() {
                self.show_settings = !self.show_settings;
            }
        });
    }

    fn preset_bar(&mut self, ui: &mut egui::Ui) {
        // Buttons are laid out from the right first so the combo box and the name field
        // take exactly the remaining width (a fixed estimate makes the panel grow every frame).
        let mut pick = None;
        ui.horizontal(|ui| {
            ui.label("Preset");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Random").on_hover_text("Random preset (Ctrl+R)").clicked() {
                    self.random_preset();
                }
                if ui.small_button("▶").on_hover_text("Next (Ctrl+Right)").clicked() {
                    self.step_preset(1);
                }
                if ui.small_button("◀").on_hover_text("Previous (Ctrl+Left)").clicked() {
                    self.step_preset(-1);
                }
                let label = self.preset_idx.and_then(|i| self.presets.get(i)).map(|p| store::preset_label(p)).unwrap_or_else(|| "–".into());
                let w = (ui.available_width() - 4.0).max(40.0);
                egui::ComboBox::from_id_salt("preset").selected_text(label).width(w).show_ui(ui, |ui| {
                    if self.presets.is_empty() {
                        ui.label(egui::RichText::new("No presets saved yet").weak());
                    }
                    for (i, p) in self.presets.iter().enumerate() {
                        if ui.selectable_label(Some(i) == self.preset_idx, store::preset_label(p)).clicked() {
                            pick = Some(i);
                        }
                    }
                });
            });
        });
        if let Some(i) = pick {
            self.load_preset(i);
        }
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(self.preset_idx.is_some(), egui::Button::new("Delete").small()).clicked() {
                    self.delete_preset();
                }
                if ui.add_enabled(self.preset_idx.is_some(), egui::Button::new("Update").small()).on_hover_text("Ctrl+U").clicked() {
                    self.update_preset();
                }
                if ui.small_button("Save").on_hover_text("Save as new preset").clicked() {
                    let n = self.preset_name.clone();
                    self.save_preset(&n);
                }
                let w = (ui.available_width() - 4.0).max(40.0);
                let r = ui.add(egui::TextEdit::singleline(&mut self.preset_name).hint_text("preset name").desired_width(w));
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let n = self.preset_name.clone();
                    self.save_preset(&n);
                }
            });
        });
    }

    fn stack(&mut self, ui: &mut egui::Ui) -> Vec<Action> {
        let mut actions = Vec::new();
        let mut remove = None;
        let mut duplicate = None;
        let mut moved: Option<(usize, usize)> = None;
        let n = self.chain.effects.len();
        let mode = self.timing.mode;
        let playhead = self.last_ft.export_progress;
        let custom_bands = self.audio.settings.custom.len();
        let blink = self.learning.as_ref().is_some_and(|l| l.started.elapsed().as_millis() / 300 % 2 == 0);
        let active_draw = self.active_draw();
        let mut pick_draw = None;
        if n == 0 {
            ui.add_space(12.0);
            ui.label(egui::RichText::new("No effects. Add one from the list on the left, or press Chaos.").weak());
        }
        for i in 0..n {
            let e = &mut self.chain.effects[i];
            let uid = e.uid;
            let frame = egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(6, 4)).fill(if e.enabled { Color32::from_gray(34) } else { Color32::from_gray(26) });
            let resp = frame
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let handle = ui.add(egui::Label::new("☰").sense(Sense::drag())).on_hover_text("Drag to reorder");
                        handle.dnd_set_drag_payload(i);
                        ui.checkbox(&mut e.enabled, "");
                        let arrow = if e.expanded { "▼" } else { "▶" };
                        if ui.small_button(arrow).clicked() {
                            e.expanded = !e.expanded;
                        }
                        match &mut self.renaming {
                            Some((ruid, buf)) if *ruid == uid => {
                                let r = ui.add(egui::TextEdit::singleline(buf).desired_width(110.0));
                                if r.lost_focus() {
                                    let s = buf.trim().to_string();
                                    e.display_name = if s.is_empty() || s == e.desc.name { None } else { Some(s) };
                                    self.renaming = None;
                                } else {
                                    r.request_focus();
                                }
                            }
                            _ => {
                                let name = egui::RichText::new(e.name().to_string()).strong();
                                let r = ui.add(egui::Label::new(name).sense(Sense::click())).on_hover_text("Double click to rename");
                                if r.double_clicked() {
                                    self.renaming = Some((uid, e.name().to_string()));
                                }
                                if r.clicked() {
                                    e.expanded = !e.expanded;
                                }
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✕").on_hover_text("Remove").clicked() {
                                remove = Some(uid);
                            }
                            if ui.small_button("⧉").on_hover_text("Duplicate").clicked() {
                                duplicate = Some(uid);
                            }
                            if ui.small_button("↺").on_hover_text("Reset parameters").clicked() {
                                for p in e.desc.params.clone() {
                                    e.reset(&p.key);
                                }
                                e.modulators.clear();
                            }
                            if PaintKind::of(&e.desc.id).is_some() {
                                let on = active_draw == Some(uid);
                                let txt = egui::RichText::new("✏").color(if on { Color32::from_rgb(120, 220, 160) } else { ui.visuals().weak_text_color() });
                                if ui.add(egui::Button::new(txt).selected(on).small()).on_hover_text("Mouse draw: paint on the preview with this effect").clicked() {
                                    pick_draw = Some(uid);
                                }
                            }
                            let lock = if e.locked { "🔒" } else { "🔓" };
                            if ui.small_button(lock).on_hover_text("Lock: kept by Chaos and preset loading").clicked() {
                                e.locked = !e.locked;
                            }
                            if i + 1 < n && ui.small_button("⏷").clicked() {
                                moved = Some((i, i + 1));
                            }
                            if i > 0 && ui.small_button("⏶").clicked() {
                                moved = Some((i, i - 1));
                            }
                        });
                    });
                    if e.expanded {
                        let learning = self.learning.as_ref().map(|l| (l.uid, l.key.as_str()));
                        let mut cx = ParamCtx { mode, live: self.live.get(&uid), playhead, actions: &mut actions, open_mods: &mut self.open_mods, custom_bands, learning, blink };
                        params_ui::effect_params(ui, e, &mut cx);
                    }
                })
                .response;
            if let Some(from) = resp.dnd_release_payload::<usize>() {
                moved = Some((*from, i));
            }
            if resp.dnd_hover_payload::<usize>().is_some() {
                ui.painter().hline(resp.rect.x_range(), resp.rect.top(), egui::Stroke::new(2.0, Color32::from_rgb(120, 220, 160)));
            }
        }
        if let Some(uid) = remove {
            self.chain.remove(uid);
        }
        if pick_draw.is_some() {
            self.draw_target = pick_draw;
        }
        if let Some(uid) = duplicate {
            self.chain.duplicate(uid);
        }
        if let Some((a, b)) = moved
            && a != b {
                self.chain.move_to(a, b);
            }
        actions
    }

    fn effect_list(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.add_filter).hint_text("search effects"));
        let filter = self.add_filter.to_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for cat in categories() {
                let items: Vec<_> = registry::available().filter(|d| d.category == cat && (filter.is_empty() || d.name.to_lowercase().contains(&filter))).collect();
                if items.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(cat).default_open(true).open(if filter.is_empty() { None } else { Some(true) }).show(ui, |ui| {
                    for d in items {
                        if ui.add(egui::Button::new(&d.name).frame(false)).clicked() {
                            let uid = self.chain.add(&d.id);
                            // a new drawing effect takes the mouse
                            if PaintKind::of(&d.id).is_some() {
                                self.draw_target = uid;
                            }
                        }
                    }
                });
            }
        });
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_rect_before_wrap();
        let (w, h) = self.out_size;
        let scale = (avail.width() / w as f32).min(avail.height() / h as f32);
        let size = Vec2::new(w as f32 * scale, h as f32 * scale);
        let rect = Rect::from_center_size(avail.center(), size);
        let orbits = self.orbit_targets();
        let drawing = orbits.is_empty() && self.active_draw().is_some();
        let resp = ui.allocate_rect(avail, if drawing || !orbits.is_empty() { Sense::drag() } else { Sense::hover() });
        if !orbits.is_empty() {
            // the 3D camera takes the mouse while its "Mouse Controls" is on
            if resp.hovered() {
                ui.ctx().set_cursor_icon(if resp.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
            }
            self.orbit_input(ui, &resp, rect, &orbits);
            self.end_stroke();
        } else {
            if drawing && resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            self.draw_input(ui, &resp, rect);
        }
        let painter = ui.painter_at(avail);
        // checkerboard for transparency
        let cell = 12.0;
        let (cols, rows) = ((rect.width() / cell).ceil() as i32, (rect.height() / cell).ceil() as i32);
        for y in 0..rows {
            for x in 0..cols {
                let c = if (x + y) % 2 == 0 { Color32::from_gray(60) } else { Color32::from_gray(90) };
                let r = Rect::from_min_size(rect.min + Vec2::new(x as f32 * cell, y as f32 * cell), Vec2::splat(cell)).intersect(rect);
                painter.rect_filled(r, 0.0, c);
            }
        }
        if let Some((_, id)) = &self.display {
            // engine output rows are bottom-up
            painter.image(*id, rect, Rect::from_min_max(Pos2::new(0.0, 1.0), Pos2::new(1.0, 0.0)), Color32::WHITE);
        }
        if self.input_path.is_none() && !matches!(self.source, Source::Live(_)) {
            painter.text(avail.center(), egui::Align2::CENTER_CENTER, "Drop an image or video here  ·  Ctrl+O", egui::FontId::proportional(18.0), Color32::from_gray(200));
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_settings;
        egui::Window::new("Settings").open(&mut open).resizable(false).show(ctx, |ui| {
            egui::Grid::new("settings").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Size");
                ui.horizontal(|ui| {
                    let (mut w, mut h) = self.out_size;
                    let rw = ui.add(egui::DragValue::new(&mut w).range(16..=MAX_DIM).suffix(" px"));
                    ui.label("×");
                    let rh = ui.add(egui::DragValue::new(&mut h).range(16..=MAX_DIM).suffix(" px"));
                    ui.checkbox(&mut self.keep_aspect, "keep aspect");
                    if rw.changed() || rh.changed() {
                        if self.keep_aspect {
                            let ar = self.input_info.as_ref().map(|i| i.width as f32 / i.height.max(1) as f32).unwrap_or(self.out_size.0 as f32 / self.out_size.1 as f32);
                            if rw.changed() {
                                h = (w as f32 / ar).round() as u32;
                            } else {
                                w = (h as f32 * ar).round() as u32;
                            }
                        }
                        if (w, h) != self.out_size {
                            self.set_size(w, h);
                        }
                    }
                });
                ui.end_row();
                ui.label("Duration");
                ui.add(egui::DragValue::new(&mut self.timing.duration).range(0.5..=3600.0).speed(0.05).suffix(" s"));
                ui.end_row();
                ui.label("Frame rate");
                egui::ComboBox::from_id_salt("fps").selected_text(format!("{}", self.timing.fps)).show_ui(ui, |ui| {
                    let mut opts = vec![60.0, 30.0, 24.0, 12.0];
                    if let Some(i) = &self.input_info
                        && i.fps > 0.0 && !opts.contains(&i.fps.round()) {
                            opts.insert(0, i.fps.round());
                        }
                    for f in opts {
                        ui.selectable_value(&mut self.timing.fps, f, format!("{f}"));
                    }
                });
                ui.end_row();
                ui.label("BPM");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.use_bpm, "sync loops");
                    ui.add_enabled(self.use_bpm, egui::DragValue::new(&mut self.bpm).range(20.0..=300.0).speed(0.2));
                    if ui.button("Tap").clicked() {
                        let now = ui.input(|i| i.time);
                        if let Some(b) = self.bpm_tap.tap(now) {
                            self.bpm = b.round();
                            self.use_bpm = true;
                        }
                    }
                });
                ui.end_row();
                ui.label("Background");
                ui.horizontal(|ui| {
                    let mut c = [self.engine.background[0], self.engine.background[1], self.engine.background[2]];
                    if ui.color_edit_button_rgb(&mut c).changed() {
                        self.engine.background[..3].copy_from_slice(&c);
                    }
                    ui.add(egui::Slider::new(&mut self.engine.background[3], 0.0..=1.0).text("opacity"));
                });
                ui.end_row();
                ui.label("Mirror input");
                ui.checkbox(&mut self.engine.flip_x, "flip horizontally");
                ui.end_row();
                if let Source::Video(v) = &self.source {
                    let clip = v.clip();
                    let len = clip.duration();
                    let mut pb = self.playback;
                    ui.label("Trim input");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut pb.trim_start).range(0.0..=len).speed(0.02).max_decimals(2).suffix(" s"));
                        ui.label("to");
                        let mut end = pb.trim_end.unwrap_or(len);
                        if ui.add(egui::DragValue::new(&mut end).range(0.0..=len).speed(0.02).max_decimals(2).suffix(" s")).changed() {
                            pb.trim_end = (end < len - 0.5 / clip.fps).then_some(end);
                        }
                        if (pb.trim_start > 0.0 || pb.trim_end.is_some()) && ui.small_button("full").clicked() {
                            pb.trim_start = 0.0;
                            pb.trim_end = None;
                        }
                    });
                    ui.end_row();
                    ui.label("Input speed");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut pb.speed).range(Playback::MIN_SPEED..=Playback::MAX_SPEED).speed(0.01).max_decimals(2).suffix("×"));
                        for d in phthora_core::Direction::ALL {
                            ui.selectable_value(&mut pb.direction, d, d.label());
                        }
                    });
                    ui.end_row();
                    if pb != self.playback {
                        self.playback = pb;
                        self.timing.duration = pb.natural_duration(clip).clamp(0.05, 3600.0);
                        self.play_time = self.play_time.min(self.timing.duration);
                        // the reader does not depend on the playback settings: keep its cache
                        if let Source::Video(v) = &mut self.source {
                            v.playback = pb;
                        }
                        self.preview.invalidate();
                    }
                }
                ui.label("Webcam");
                ui.horizontal(|ui| {
                    let cams = self.cameras();
                    let cur = self.settings.camera.clone().filter(|p| cams.iter().any(|d| &d.path == p)).or_else(|| cams.first().map(|d| d.path.clone()));
                    let label = |p: &Option<PathBuf>| p.as_ref().and_then(|p| cams.iter().find(|d| &d.path == p)).map(|d| d.name.clone()).unwrap_or_else(|| "none found".into());
                    let mut pick = cur.clone();
                    egui::ComboBox::from_id_salt("camera").selected_text(label(&cur)).width(200.0).show_ui(ui, |ui| {
                        for d in &cams {
                            ui.selectable_value(&mut pick, Some(d.path.clone()), format!("{} ({})", d.name, d.path.display()));
                        }
                    });
                    if pick != cur {
                        self.settings.camera = pick;
                        if matches!(self.source, Source::Live(_)) {
                            self.start_webcam(None);
                        }
                    }
                    if ui.small_button("↻").on_hover_text("Look for devices again").clicked() {
                        self.devices = phthora_media::list_devices();
                    }
                });
                ui.end_row();
                ui.label("Virtual camera");
                ui.horizontal(|ui| {
                    let outs: Vec<VideoDevice> = self.devices.iter().filter(|d| d.loopback).cloned().collect();
                    let mut on = self.settings.virtual_cam.is_some();
                    let changed = ui
                        .add_enabled(!outs.is_empty(), egui::Checkbox::new(&mut on, "send the preview"))
                        .on_disabled_hover_text("Needs a v4l2loopback device (modprobe v4l2loopback)")
                        .changed();
                    if changed {
                        self.settings.virtual_cam = on.then(|| outs.iter().find(|d| Some(&d.path) != self.settings.camera.as_ref()).or(outs.first()).map(|d| d.path.clone())).flatten();
                    }
                    if let Some(cur) = self.settings.virtual_cam.clone() {
                        let mut pick = cur.clone();
                        egui::ComboBox::from_id_salt("vcam").selected_text(cur.display().to_string()).show_ui(ui, |ui| {
                            for d in &outs {
                                ui.selectable_value(&mut pick, d.path.clone(), format!("{} ({})", d.name, d.path.display()));
                            }
                        });
                        if pick != cur {
                            self.settings.virtual_cam = Some(pick);
                            self.vcam = None;
                        }
                    }
                    if changed {
                        // switched: (re)open on the next rendered frame
                        self.vcam = None;
                    }
                });
                ui.end_row();
                ui.label("Encoding");
                let hw = self.vaapi.load(Ordering::Relaxed);
                ui.add_enabled(hw, egui::Checkbox::new(&mut self.settings.hw_encode, "use the GPU for MP4 (VAAPI)"))
                    .on_disabled_hover_text("No VAAPI H.264 encoder found");
                ui.end_row();
                ui.label("Presets");
                if ui.button("Open preset folder").clicked() {
                    let _ = std::process::Command::new("xdg-open").arg(store::preset_dir()).spawn();
                }
                ui.end_row();
            });
        });
        if open && !self.show_settings || self.devices.is_empty() && open {
            self.devices = phthora_media::list_devices();
        }
        if open != self.show_settings || !open {
            self.settings.background = self.engine.background;
            if matches!(self.source, Source::Live(_)) {
                self.settings.flip_webcam = self.engine.flip_x;
            } else {
                self.settings.flip_x = self.engine.flip_x;
            }
            if self.show_settings && !open {
                self.settings.save();
            }
        }
        self.show_settings = open;
    }

    fn export_window(&mut self, ctx: &egui::Context) {
        if !self.export.open {
            return;
        }
        let mut open = true;
        let mut start = false;
        let mut cancel = false;
        let live = matches!(self.source, Source::Live(_));
        egui::Window::new("Export").open(&mut open).resizable(false).show(ctx, |ui| {
            if live {
                ui.label("The webcam has no fixed length: record it with ● Rec,\nthe recording opens here for the export.");
                return;
            }
            let running = self.export.running.is_some();
            ui.add_enabled_ui(!running, |ui| {
                egui::Grid::new("export").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Format");
                    ui.horizontal(|ui| {
                        if self.timing.mode == OutputMode::Image {
                            ui.selectable_value(&mut self.export.format, Format::Png, "PNG");
                        } else {
                            ui.selectable_value(&mut self.export.format, Format::Mp4, "MP4");
                            ui.selectable_value(&mut self.export.format, Format::WebM, "WebM");
                            ui.selectable_value(&mut self.export.format, Format::Gif, "GIF");
                        }
                    });
                    ui.end_row();
                    ui.label("Size");
                    ui.label(format!("{} × {}", self.out_size.0, self.out_size.1));
                    ui.end_row();
                    if self.timing.mode == OutputMode::Video {
                        ui.label("Duration");
                        ui.add(egui::DragValue::new(&mut self.timing.duration).range(0.5..=3600.0).speed(0.05).suffix(" s"));
                        ui.end_row();
                        ui.label("Frame rate");
                        ui.label(format!("{} fps  ({} frames)", self.timing.fps, self.timing.frame_count()));
                        ui.end_row();
                        if self.export.format != Format::Gif {
                            ui.label("Quality");
                            ui.horizontal(|ui| {
                                for q in [Quality::Ultra, Quality::High, Quality::Medium, Quality::Low] {
                                    ui.selectable_value(&mut self.export.quality, q, quality_name(q));
                                }
                            });
                            ui.end_row();
                            ui.label("Audio");
                            ui.horizontal(|ui| {
                                let what = match (self.audio.export_source(), self.audio.src) {
                                    (Some(a), _) => a.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                    (None, Src::Mic) => "none (microphone is live only)".into(),
                                    (None, _) if !self.audio.include_in_export && self.audio.track.is_some() => "not included".into(),
                                    (None, _) => "none".into(),
                                };
                                ui.label(what);
                                if ui.small_button("Audio…").clicked() {
                                    self.audio.open = true;
                                }
                            });
                            ui.end_row();
                        }
                        if self.export.format == Format::Mp4 && self.engine.background[3] < 1.0 {
                            ui.label("");
                            ui.label(egui::RichText::new("MP4 has no transparency: exported on an opaque background").small().weak());
                            ui.end_row();
                        }
                    }
                });
            });
            ui.separator();
            if let Some((_, p, t0)) = &self.export.running {
                let f = p.fraction();
                let el = t0.elapsed().as_secs_f32();
                let eta = if f > 0.01 { el / f - el } else { 0.0 };
                ui.add(egui::ProgressBar::new(f).show_percentage().text(format!("{:.0}%  ·  {:.0} s left", f * 100.0, eta)));
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            } else if ui.button("Export…").clicked() {
                start = true;
            }
        });
        if cancel
            && let Some((_, p, _)) = &self.export.running {
                p.cancel.store(true, Ordering::Relaxed);
            }
        if start {
            self.start_export();
        }
        if !open && self.export.running.is_none() {
            self.export.open = false;
        }
    }

    fn handle_actions(&mut self, actions: Vec<Action>) {
        for a in actions {
            match a {
                Action::ClearPaint { uid } => {
                    if let Some(e) = self.chain.get_mut(uid) {
                        e.data.remove(paint::DATA_KEY);
                    }
                    if let Some(c) = self.engine.paint.get_mut(&uid) {
                        c.load_data_url("");
                    }
                }
                Action::LearnMidi { uid, key } => {
                    let since = self.midi.seq();
                    self.learning = Some(Learn { uid, key, since, started: Instant::now() });
                    let devices = self.midi.devices();
                    if devices.is_empty() {
                        self.notify("MIDI learn: no MIDI input connected");
                    } else {
                        self.notify(format!("MIDI learn: move a controller ({})", devices.join(", ")));
                    }
                }
                Action::PickFile { uid } => {
                    let Some(e) = self.chain.get_mut(uid) else { continue };
                    let image_only = e.desc.id == "mask";
                    let mut d = rfd::FileDialog::new();
                    d = if image_only {
                        d.add_filter("Image", &["png", "jpg", "jpeg", "webp", "bmp"])
                    } else {
                        d.add_filter("Media", &["png", "jpg", "jpeg", "webp", "mp4", "mov", "webm", "gif", "mkv"])
                    };
                    if let Some(p) = d.pick_file() {
                        let s = p.to_string_lossy().to_string();
                        e.set("path", Value::Text(s));
                        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        if e.desc.param("fileName").is_some() {
                            e.set("fileName", Value::Text(name));
                        }
                        self.engine.assets.forget_image(&p);
                    }
                }
            }
        }
    }
}

fn parse_quality(s: &str) -> Quality {
    match s {
        "ultra" => Quality::Ultra,
        "medium" => Quality::Medium,
        "low" => Quality::Low,
        _ => Quality::High,
    }
}

fn quality_name(q: Quality) -> &'static str {
    match q {
        Quality::Ultra => "ultra",
        Quality::High => "high",
        Quality::Medium => "medium",
        Quality::Low => "low",
    }
}

impl eframe::App for PhthoraApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // dropped files
        let dropped: Option<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| { let p = f.path(); (!p.as_os_str().is_empty()).then(|| p.to_path_buf()) }));
        if let Some(p) = dropped {
            if p.to_string_lossy().ends_with(store::PRESET_EXT) {
                if let Ok(s) = std::fs::read_to_string(&p)
                    && let Ok((fx, _)) = phthora_core::preset::from_json(&s) {
                        self.chain.replace(fx, true);
                    }
            } else {
                self.load_input(&p);
            }
        }
        if self.preset_scan.elapsed().as_secs_f32() > 2.0 {
            self.rescan_presets();
            self.preset_scan = Instant::now();
        }
        self.shortcuts(&ctx);
        self.poll_export();
        self.tick();

        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui);
            ui.add_space(2.0);
        });
        let status_visible = self.status_time.map(|t| t.elapsed().as_secs_f32() < 8.0).unwrap_or(false);
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if status_visible {
                    ui.label(egui::RichText::new(&self.status).small());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mode = if self.timing.mode == OutputMode::Image { "image" } else { "video" };
                    ui.label(egui::RichText::new(format!("{}×{} · {} · {} fps", self.out_size.0, self.out_size.1, mode, self.timing.fps)).small().weak());
                });
            });
        });
        egui::Panel::left("effects").resizable(true).default_size(170.0).show(ui, |ui| {
            ui.heading("Add effect");
            self.effect_list(ui);
        });
        let mut actions = Vec::new();
        egui::Panel::right("stack").resizable(true).default_size(360.0).min_size(300.0).show(ui, |ui| {
            self.preset_bar(ui);
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                actions = self.stack(ui);
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            self.preview(ui);
        });
        self.handle_actions(actions);
        self.settings_window(&ctx);
        self.export_window(&ctx);
        let input_audio = self.input_audio_usable();
        self.audio.window(&ctx, input_audio);
        // an edit is complete once the pointer is released and no text field has focus
        if !ctx.input(|i| i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.history.observe(&self.chain.effects);
        }
        // keep animating at the output frame rate
        if self.audio.loading() || self.learning.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        // MIDI controllers change values without any window input
        let midi_mapped = self.chain.effects.iter().any(|e| e.enabled && e.modulators.values().any(|m| m.enabled && m.shape == phthora_core::modulator::Shape::Midi));
        if midi_mapped && self.timing.mode == OutputMode::Video {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
        if self.waiting {
            self.waiting = false;
            ctx.request_repaint_after(std::time::Duration::from_millis(4));
        } else if self.timing.mode == OutputMode::Video && self.playing || self.export.running.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(1.0 / self.timing.fps.max(1.0)));
        }
    }

    fn on_exit(&mut self) {
        self.audio.stop();
        self.settings.audio = self.audio.settings.clone();
        self.settings.background = self.engine.background;
        if matches!(self.source, Source::Live(_)) {
            self.settings.flip_webcam = self.engine.flip_x;
        } else {
            self.settings.flip_x = self.engine.flip_x;
        }
        self.settings.save();
    }
}
