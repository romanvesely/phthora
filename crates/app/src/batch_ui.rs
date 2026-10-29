//! The batch export window: a list of files, an output folder and optional settings that
//! hold for every file. The work itself is `batch::run_batch`, started by the main window.

use crate::batch::{self, BatchProgress, Overrides, Report};
use eframe::egui::{self, Color32, Rect};
use phthora_media::Format;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// File extensions the window takes (the same as the editor's Open dialog).
pub const MEDIA_EXTENSIONS: [&str; 12] = ["mp4", "mov", "webm", "ogg", "avi", "mkv", "m4v", "gif", "png", "jpg", "jpeg", "webp"];

/// Formats offered for all files (a PNG sequence is a folder: command line only).
const FORMATS: [Format; 7] = [Format::Mp4, Format::WebM, Format::Gif, Format::Webp, Format::Apng, Format::Png, Format::Jpg];

/// The running batch: worker thread, progress, start time.
pub struct Running {
    pub handle: std::thread::JoinHandle<Report>,
    pub progress: BatchProgress,
    pub started: Instant,
}

/// What the window asks the main window to do.
pub enum Cmd {
    Start,
    Cancel,
    OpenFolder(PathBuf),
}

/// Values the settings start with: the editor's own.
pub struct Defaults {
    pub size: (u32, u32),
    pub duration: f64,
    pub fps: f64,
    pub format: Format,
    pub out_dir: PathBuf,
}

pub struct BatchUi {
    pub open: bool,
    pub files: Vec<PathBuf>,
    pub out_dir: Option<PathBuf>,
    /// size, length, frame rate and format follow each file
    pub match_input: bool,
    size: (u32, u32),
    duration: f64,
    fps: f64,
    format: Format,
    use_size: bool,
    use_duration: bool,
    use_fps: bool,
    use_format: bool,
    pub running: Option<Running>,
    /// result of the last batch
    pub report: Option<Report>,
    /// where the window was drawn last, to tell a drop on it from a drop on the editor
    rect: Option<Rect>,
    /// folder of the last file dialog
    pub input_dir: Option<PathBuf>,
}

impl BatchUi {
    pub fn new() -> Self {
        Self {
            open: false,
            files: Vec::new(),
            out_dir: None,
            match_input: true,
            size: (1280, 720),
            duration: 4.0,
            fps: 30.0,
            format: Format::Mp4,
            use_size: true,
            use_duration: true,
            use_fps: true,
            use_format: true,
            running: None,
            report: None,
            rect: None,
            input_dir: None,
        }
    }

    /// Open the window. The settings start from the editor's, the folder from the last export.
    pub fn open_with(&mut self, d: Defaults) {
        if !self.open {
            self.size = d.size;
            self.duration = d.duration;
            self.fps = d.fps;
            self.format = if FORMATS.contains(&d.format) { d.format } else { Format::Mp4 };
            self.out_dir.get_or_insert(d.out_dir);
        }
        self.open = true;
    }

    /// Add files that are media (by extension) and not in the list yet; returns how many were new.
    pub fn add(&mut self, paths: impl IntoIterator<Item = PathBuf>) -> usize {
        let before = self.files.len();
        for p in paths {
            let media = p.extension().is_some_and(|e| MEDIA_EXTENSIONS.iter().any(|m| e.eq_ignore_ascii_case(m)));
            if media && p.is_file() && !self.files.contains(&p) {
                self.files.push(p);
            }
        }
        self.files.len() - before
    }

    /// Settings that hold for every file; none while "Match input" is on.
    pub fn overrides(&self) -> Overrides {
        if self.match_input {
            return Overrides::default();
        }
        Overrides {
            size: self.use_size.then_some(self.size),
            duration: self.use_duration.then_some(self.duration),
            fps: self.use_fps.then_some(self.fps),
            format: self.use_format.then_some(self.format),
        }
    }

    /// A file dropped now belongs to this window: it is open and the pointer is on it (or unknown).
    pub fn wants_drop(&self, ctx: &egui::Context) -> bool {
        if !self.open {
            return false;
        }
        match (ctx.input(|i| i.pointer.latest_pos()), self.rect) {
            (Some(p), Some(r)) => r.contains(p),
            _ => true,
        }
    }

    /// Fold a finished batch into the window; returns the line for the status bar.
    pub fn finish(&mut self, report: Report) -> String {
        let line = batch::summary(&report);
        self.report = Some(report);
        line
    }

    /// Draw the window. `can_start` is false while an export of the editor runs.
    pub fn show(&mut self, ctx: &egui::Context, can_start: bool) -> Option<Cmd> {
        if !self.open {
            return None;
        }
        let mut cmd = None;
        let mut open = true;
        let running = self.running.is_some();
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let resp = egui::Window::new("Batch export").open(&mut open).resizable(false).default_width(420.0).show(ctx, |ui| {
            ui.set_min_width(400.0);
            ui.add_enabled_ui(!running, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Add files…").on_hover_text("Or drop files on this window").clicked() {
                        let mut d = rfd::FileDialog::new().add_filter("Media", &MEDIA_EXTENSIONS);
                        if let Some(dir) = &self.input_dir {
                            d = d.set_directory(dir);
                        }
                        if let Some(files) = d.pick_files() {
                            self.input_dir = files.first().and_then(|f| f.parent()).map(Path::to_path_buf);
                            self.add(files);
                        }
                    }
                    if ui.add_enabled(!self.files.is_empty(), egui::Button::new("Clear")).clicked() {
                        self.files.clear();
                    }
                    ui.label(egui::RichText::new(format!("{} file{}", self.files.len(), if self.files.len() == 1 { "" } else { "s" })).weak());
                });
                let mut remove = None;
                let frame = egui::Frame::group(ui.style()).stroke(egui::Stroke::new(if hovering { 2.0 } else { 1.0 }, if hovering { Color32::from_rgb(235, 190, 60) } else { ui.visuals().widgets.noninteractive.bg_stroke.color }));
                frame.show(ui, |ui| {
                    ui.set_min_size(egui::vec2(ui.available_width(), 90.0));
                    if self.files.is_empty() {
                        ui.label(egui::RichText::new("Drop image and video files here").weak());
                    }
                    egui::ScrollArea::vertical().max_height(180.0).auto_shrink([false, true]).show(ui, |ui| {
                        for (i, f) in self.files.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.small_button("✕").on_hover_text("Remove").clicked() {
                                    remove = Some(i);
                                }
                                let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                ui.add(egui::Label::new(name).truncate()).on_hover_text(f.display().to_string());
                            });
                        }
                    });
                });
                if let Some(i) = remove {
                    self.files.remove(i);
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Output folder");
                    if ui.button("Choose…").clicked()
                        && let Some(d) = rfd::FileDialog::new().set_directory(self.out_dir.clone().unwrap_or_default()).pick_folder() {
                            self.out_dir = Some(d);
                        }
                    let text = self.out_dir.as_ref().map(|d| d.display().to_string()).unwrap_or_else(|| "not chosen".into());
                    ui.add(egui::Label::new(egui::RichText::new(text).weak()).truncate());
                });
                ui.add_space(4.0);
                ui.checkbox(&mut self.match_input, "Match input").on_hover_text("Size, length, frame rate and format follow each file");
                if !self.match_input {
                    egui::Grid::new("batch_overrides").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.checkbox(&mut self.use_size, "Size");
                        ui.add_enabled_ui(self.use_size, |ui| {
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut self.size.0).range(16..=4096).suffix(" px"));
                                ui.label("×");
                                ui.add(egui::DragValue::new(&mut self.size.1).range(16..=4096).suffix(" px"));
                            });
                        });
                        ui.end_row();
                        ui.checkbox(&mut self.use_duration, "Duration").on_hover_text("A still image becomes a video of this length");
                        ui.add_enabled(self.use_duration, egui::DragValue::new(&mut self.duration).range(0.05..=3600.0).speed(0.05).suffix(" s"));
                        ui.end_row();
                        ui.checkbox(&mut self.use_fps, "Frame rate");
                        ui.add_enabled(self.use_fps, egui::DragValue::new(&mut self.fps).range(1.0..=120.0).speed(0.1).suffix(" fps"));
                        ui.end_row();
                        ui.checkbox(&mut self.use_format, "Format");
                        ui.add_enabled_ui(self.use_format, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for f in FORMATS {
                                    let missing = phthora_media::missing_encoder(f);
                                    let r = ui.add_enabled_ui(missing.is_none(), |ui| ui.selectable_label(self.format == f, f.label())).inner;
                                    if let Some(enc) = missing {
                                        r.on_disabled_hover_text(format!("This ffmpeg has no {enc} encoder"));
                                    } else if r.clicked() {
                                        self.format = f;
                                    }
                                }
                            });
                        });
                        ui.end_row();
                    });
                    ui.label(egui::RichText::new("What is not ticked follows each file.").small().weak());
                }
                ui.label(egui::RichText::new("Uses the current effects, background and quality (Export window).").small().weak());
            });
            ui.separator();
            if let Some(r) = &self.running {
                let total = r.progress.total.load(std::sync::atomic::Ordering::Relaxed);
                let index = r.progress.index.load(std::sync::atomic::Ordering::Relaxed);
                let name = r.progress.name.lock().map(|n| n.clone()).unwrap_or_default();
                ui.label(format!("File {index}/{total}: {name}"));
                ui.add(egui::ProgressBar::new(r.progress.frames.fraction().min(1.0)).show_percentage());
                let f = r.progress.fraction();
                let el = r.started.elapsed().as_secs_f32();
                let eta = if f > 0.01 { el / f - el } else { 0.0 };
                ui.add(egui::ProgressBar::new(f).text(format!("all  ·  {:.0}%  ·  {:.0} s left", f * 100.0, eta)));
                if ui.button("Cancel").clicked() {
                    cmd = Some(Cmd::Cancel);
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            } else {
                let ready = !self.files.is_empty() && self.out_dir.is_some() && can_start;
                let r = ui.add_enabled(ready, egui::Button::new("Export all"));
                if r.clicked() {
                    cmd = Some(Cmd::Start);
                }
                if !can_start {
                    ui.label(egui::RichText::new("An export is running.").small().weak());
                }
            }
            if let Some(rep) = &self.report
                && !running {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(batch::summary(rep));
                        if !rep.done.is_empty()
                            && let Some(d) = &self.out_dir
                            && ui.small_button("Open folder").clicked() {
                                cmd = Some(Cmd::OpenFolder(d.clone()));
                            }
                    });
                    if !rep.failed.is_empty() {
                        egui::ScrollArea::vertical().id_salt("batch_failed").max_height(140.0).show(ui, |ui| {
                            for f in &rep.failed {
                                let name = f.input.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                let last = f.error.lines().last().unwrap_or(&f.error);
                                ui.label(egui::RichText::new(format!("{name}: {last}")).color(Color32::from_rgb(240, 120, 110))).on_hover_text(&f.error);
                            }
                        });
                    }
                }
        });
        self.rect = resp.map(|r| r.response.rect);
        if !open && !running {
            self.open = false;
        }
        cmd
    }
}
