//! Audio in the editor: which track drives audio modulators and is exported, its analysis,
//! preview playback (the preview clock while it plays) and live microphone input.

use eframe::egui;
use phthora_audio::{Analyzer, AudioSettings, AudioTrack, BandDef, Follower, FrameLevels, MicInput, Player, Program};
use phthora_core::modulator::{LiveInputs, MAX_BANDS};
use phthora_media::AudioSource;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Src {
    None,
    /// the input video's own audio
    Input,
    File,
    /// live microphone (preview only)
    Mic,
}

pub struct AudioState {
    pub src: Src,
    pub file: Option<PathBuf>,
    /// seconds skipped at the start of the audio file
    pub file_start: f64,
    pub settings: AudioSettings,
    pub muted: bool,
    pub include_in_export: bool,
    pub open: bool,
    /// analysed track with its used part set
    pub track: Option<AudioTrack>,
    track_path: Option<PathBuf>,
    loading: Option<(PathBuf, JoinHandle<Result<AudioTrack, String>>)>,
    pub error: Option<String>,
    player: Option<Player>,
    player_tried: bool,
    mic: Option<MicInput>,
    analyzer: Analyzer,
    live: Follower,
    frames: FrameLevels,
    /// current level of the "all" band (meter)
    pub meter: f32,
    pub spectrum: Vec<f32>,
    /// level of the "all" band over the output, for the keyframe editor (key, levels)
    envelope: Option<(String, Arc<Vec<f32>>)>,
}

impl AudioState {
    pub fn new(settings: AudioSettings) -> Self {
        Self {
            src: Src::Input,
            file: None,
            file_start: 0.0,
            settings,
            muted: false,
            include_in_export: true,
            open: false,
            track: None,
            track_path: None,
            loading: None,
            error: None,
            player: None,
            player_tried: false,
            mic: None,
            analyzer: Analyzer::new(),
            live: Follower::default(),
            frames: FrameLevels::default(),
            meter: 0.0,
            spectrum: Vec::new(),
            envelope: None,
        }
    }

    /// Audio file the current source reads, if any.
    fn wanted(&self, input: Option<&Path>, input_audio: bool) -> Option<PathBuf> {
        match self.src {
            Src::Input if input_audio => input.map(Path::to_path_buf),
            Src::File => self.file.clone(),
            _ => None,
        }
    }

    /// Keep the analysed track in step with the source; `trim` is the input's used part.
    pub fn sync(&mut self, input: Option<&Path>, input_audio: bool, trim: (f64, Option<f64>)) {
        let want = self.wanted(input, input_audio);
        if want != self.track_path && self.loading.as_ref().map(|(p, _)| Some(p)) != Some(want.as_ref()) {
            self.track = None;
            self.track_path = None;
            self.loading = want.clone().map(|p| {
                let q = p.clone();
                (p, std::thread::spawn(move || AudioTrack::load(&q)))
            });
        }
        if self.loading.as_ref().is_some_and(|(_, h)| h.is_finished()) {
            let (p, h) = self.loading.take().unwrap();
            match h.join().unwrap_or_else(|_| Err("audio analysis crashed".into())) {
                Ok(t) => {
                    self.track = Some(t);
                    self.track_path = Some(p);
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(format!("Cannot read audio: {}", e.lines().last().unwrap_or(&e)));
                    self.track_path = Some(p);
                }
            }
        }
        let (start, end) = match self.src {
            Src::File => (self.file_start, None),
            _ => trim,
        };
        if let Some(t) = &mut self.track {
            t.start = start;
            t.end = end;
        }
        if self.src == Src::Mic && self.mic.is_none() {
            match MicInput::open() {
                Ok(m) => self.mic = Some(m),
                Err(e) => {
                    self.error = Some(format!("Microphone: {e}"));
                    self.src = Src::None;
                }
            }
        } else if self.src != Src::Mic {
            self.mic = None;
        }
    }

    /// Loudness (RMS) of the audio track over an output `duration` seconds long, in evenly
    /// spaced slices, or None without an analysed track.
    pub fn envelope(&mut self, duration: f64) -> Option<Arc<Vec<f32>>> {
        const POINTS: usize = 200;
        let track = self.track.as_ref()?;
        let key = format!("{:p} {} {:?} {duration}", Arc::as_ptr(&track.pcm), track.start, track.end);
        if let Some((k, env)) = &self.envelope
            && *k == key
        {
            return Some(env.clone());
        }
        let rate = phthora_audio::SAMPLE_RATE as f64;
        let slice = duration / POINTS as f64;
        let n = ((slice * rate) as usize).max(1);
        // every fourth sample is plenty for a level
        let env: Arc<Vec<f32>> = Arc::new(
            (0..POINTS)
                .map(|i| {
                    let start = (track.track_time(i as f64 * slice) * rate) as usize;
                    let (sum, count) = (start..start + n).step_by(4).filter_map(|j| track.pcm.get(j)).fold((0.0f32, 0u32), |(s, c), v| (s + v * v, c + 1));
                    if count > 0 { (sum / count as f32).sqrt() } else { 0.0 }
                })
                .collect(),
        );
        self.envelope = Some((key, env.clone()));
        Some(env)
    }

    pub fn loading(&self) -> bool {
        self.loading.is_some()
    }

    /// Levels for output frame `n` (live microphone: the current levels).
    pub fn inputs(&mut self, n: u64, fps: f64) -> LiveInputs {
        let levels = match &self.mic {
            Some(m) => {
                let d = self.analyzer.analyze(&m.latest());
                self.live.step(Some(&d), &self.settings).clone()
            }
            None => self.frames.at(n, fps, self.track.as_ref(), &self.settings).clone(),
        };
        self.meter = levels.bands[0];
        self.spectrum = levels.spectrum;
        let mut li = LiveInputs::default();
        for (i, v) in levels.bands.iter().take(MAX_BANDS).enumerate() {
            li.audio[i] = *v;
        }
        li
    }

    /// Drive preview playback at picture time `t`. The picture keeps the wall clock;
    /// the audio is moved back in line when it drifts more than 50 ms (or its device is not
    /// paced by real time).
    pub fn play(&mut self, playing: bool, t: f64, loop_len: f64) {
        let Some(tr) = self.track.as_ref().filter(|_| self.src != Src::Mic) else {
            self.stop();
            return;
        };
        if self.player.is_none() && !self.player_tried {
            self.player_tried = true;
            match Player::new() {
                Ok(p) => self.player = Some(p),
                Err(e) => eprintln!("no audio playback: {e}"),
            }
        }
        let Some(p) = self.player.as_ref() else { return };
        p.set_program(Some(Program { pcm: tr.pcm.clone(), start: tr.start, span: tr.span(), loop_len }));
        p.set_muted(self.muted);
        p.set_playing(playing);
        let d = (p.position() - t).abs();
        let drift = d.min((loop_len - d).abs());
        if !playing || drift > 0.05 {
            p.seek(t);
        }
    }

    pub fn stop(&self) {
        if let Some(p) = &self.player {
            p.set_playing(false);
        }
    }

    /// Audio for the exported file.
    pub fn export_source(&self) -> Option<AudioSource> {
        if !self.include_in_export {
            return None;
        }
        let t = self.track.as_ref()?;
        Some(AudioSource { path: self.track_path.clone()?, start: t.start, end: t.end })
    }

    pub fn window(&mut self, ctx: &egui::Context, input_audio: bool) {
        let mut open = self.open;
        egui::Window::new("Audio").open(&mut open).resizable(false).show(ctx, |ui| {
            egui::Grid::new("audio").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Source");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.src, Src::None, "none");
                    ui.add_enabled_ui(input_audio, |ui| ui.selectable_value(&mut self.src, Src::Input, "input"))
                        .response
                        .on_disabled_hover_text("The input has no audio, or it is reversed or re-timed");
                    ui.selectable_value(&mut self.src, Src::File, "file");
                    ui.selectable_value(&mut self.src, Src::Mic, "microphone").on_hover_text("Live input, preview only");
                });
                ui.end_row();
                if self.src == Src::File {
                    ui.label("File");
                    ui.horizontal(|ui| {
                        let name = self.file.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "–".into());
                        ui.label(name);
                        if ui.button("Choose…").clicked()
                            && let Some(p) = rfd::FileDialog::new().add_filter("Audio", &["mp3", "wav", "ogg", "m4a", "aac", "flac", "opus", "mp4", "mov", "webm"]).pick_file() {
                                self.file = Some(p);
                                self.file_start = 0.0;
                            }
                    });
                    ui.end_row();
                    ui.label("Start at");
                    let len = self.track.as_ref().map(|t| t.duration()).unwrap_or(3600.0);
                    ui.add(egui::DragValue::new(&mut self.file_start).range(0.0..=len).speed(0.05).max_decimals(2).suffix(" s"));
                    ui.end_row();
                }
                ui.label("Gain");
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut self.settings.gain, 0.0..=5.0).step_by(0.01));
                    let (r, p) = ui.allocate_painter(egui::vec2(60.0, 10.0), egui::Sense::hover());
                    p.rect_filled(r.rect, 2.0, egui::Color32::from_gray(50));
                    let mut f = r.rect;
                    f.set_width(r.rect.width() * self.meter.clamp(0.0, 1.0));
                    p.rect_filled(f, 2.0, egui::Color32::from_rgb(120, 220, 160));
                });
                ui.end_row();
                ui.label("Smoothing");
                ui.add(egui::Slider::new(&mut self.settings.smoothing, 0.0..=1.0).step_by(0.01));
                ui.end_row();
                ui.label("Reduce noise");
                ui.checkbox(&mut self.settings.reduce_noise, "").on_hover_text("Quiet signals are pushed down, loud ones up");
                ui.end_row();
                ui.label("Preview");
                ui.checkbox(&mut self.muted, "mute");
                ui.end_row();
                ui.label("Export");
                ui.checkbox(&mut self.include_in_export, "include the audio track");
                ui.end_row();
            });
            ui.separator();
            ui.label("Extra bands (for audio modulators)");
            let mut remove = None;
            for (i, b) in self.settings.custom.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("custom{}", i + 1));
                    ui.add(egui::DragValue::new(&mut b.lo_hz).range(0.0..=22050.0).speed(5.0).suffix(" Hz"));
                    ui.label("–");
                    ui.add(egui::DragValue::new(&mut b.hi_hz).range(0.0..=22050.0).speed(5.0).suffix(" Hz"));
                    ui.add(egui::DragValue::new(&mut b.gain).range(0.0..=5.0).speed(0.01).prefix("×"));
                    if ui.add(crate::icons::small(crate::icons::Icon::Close)).clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                self.settings.custom.remove(i);
            }
            if self.settings.custom.len() < MAX_BANDS - 4 && ui.small_button("Add band").clicked() {
                self.settings.custom.push(BandDef { lo_hz: 200.0, hi_hz: 400.0, gain: 1.0 });
            }
            if let Some(e) = &self.error {
                ui.colored_label(egui::Color32::from_rgb(240, 120, 100), e);
            }
            if self.loading() {
                ui.label(egui::RichText::new("Analysing audio…").weak());
            }
        });
        self.open = open;
    }
}
