//! phthora: real-time GPU effect mixer for images, video and GIFs.

mod app;
mod params_ui;
mod render;
mod store;
mod video;

use phthora_core::{EffectInstance, OutputMode, TimingSettings};
use phthora_media::{Format, Quality};
use std::path::PathBuf;

const USAGE: &str = "usage:
  phthora [FILE]                     open the editor (optionally with an image or video)
  phthora render [INPUT] [options]   render without a window
options for render:
  --preset FILE      effect chain (.phthora.json)
  --effect ID,...    add effects with default settings (see --list)
  --out FILE         output file (format from extension: mp4, webm, gif, png)
  --duration SEC     output length (default: input length, 4 s for stills)
  --fps N            frame rate (default: input rate or 30)
  --size WxH         output size (default: input size, max 4096)
  --quality Q        ultra | high | medium | low
  --bpm N            sync effect loops to a tempo
  --list             list effect ids";

fn arg_value(args: &[String], i: &mut usize) -> Result<String, String> {
    *i += 1;
    args.get(*i).cloned().ok_or_else(|| format!("missing value for {}", args[*i - 1]))
}

fn cli_render(args: &[String]) -> Result<(), String> {
    let mut input: Option<PathBuf> = None;
    let mut preset: Option<PathBuf> = None;
    let mut effects_arg: Vec<String> = Vec::new();
    let mut out: Option<PathBuf> = None;
    let mut duration: Option<f64> = None;
    let mut fps: Option<f64> = None;
    let mut size: Option<(u32, u32)> = None;
    let mut quality = Quality::High;
    let mut bpm = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--preset" => preset = Some(arg_value(args, &mut i)?.into()),
            "--effect" => effects_arg.extend(arg_value(args, &mut i)?.split(',').map(|s| s.trim().to_string())),
            "--out" => out = Some(arg_value(args, &mut i)?.into()),
            "--duration" => duration = Some(arg_value(args, &mut i)?.parse().map_err(|_| "bad --duration")?),
            "--fps" => fps = Some(arg_value(args, &mut i)?.parse().map_err(|_| "bad --fps")?),
            "--bpm" => bpm = Some(arg_value(args, &mut i)?.parse().map_err(|_| "bad --bpm")?),
            "--size" => {
                let v = arg_value(args, &mut i)?;
                let (w, h) = v.split_once('x').ok_or("bad --size, use WxH")?;
                size = Some((w.parse().map_err(|_| "bad width")?, h.parse().map_err(|_| "bad height")?));
            }
            "--quality" => {
                quality = match arg_value(args, &mut i)?.as_str() {
                    "ultra" => Quality::Ultra,
                    "high" => Quality::High,
                    "medium" => Quality::Medium,
                    "low" => Quality::Low,
                    q => return Err(format!("unknown quality {q}")),
                }
            }
            "--list" => {
                use std::io::Write;
                let mut out = std::io::stdout().lock();
                for d in phthora_core::registry::available() {
                    // stop quietly when the reader goes away (e.g. piped into head)
                    if writeln!(out, "{:16} {:24} {}", d.id, d.category, d.name).is_err() {
                        break;
                    }
                }
                return Ok(());
            }
            _ if a.starts_with("--") => return Err(format!("unknown option {a}\n{USAGE}")),
            _ => input = Some(a.into()),
        }
        i += 1;
    }
    let out = out.ok_or("--out is required")?;
    let ext = out.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let format = match ext.as_str() {
        "mp4" => Format::Mp4,
        "webm" => Format::WebM,
        "gif" => Format::Gif,
        "png" => Format::Png,
        _ => return Err("output must end in .mp4, .webm, .gif or .png".into()),
    };
    let mut effects = Vec::new();
    if let Some(p) = &preset {
        let s = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let (fx, warnings) = phthora_core::preset::from_json(&s).map_err(|e| e.to_string())?;
        for w in warnings {
            eprintln!("preset: {w}");
        }
        effects.extend(fx);
    }
    for id in &effects_arg {
        let mut e = EffectInstance::by_id(id).filter(|e| e.desc.available()).ok_or_else(|| format!("unknown effect {id} (see --list)"))?;
        e.enabled = true;
        effects.push(e);
    }
    let info = match &input {
        Some(p) => Some(phthora_media::probe(p).map_err(|e| e.to_string())?),
        None => None,
    };
    let (w, h) = size.unwrap_or_else(|| info.as_ref().map(|i| phthora_media::fit_size(i.width, i.height, 4096)).unwrap_or((1280, 720)));
    let in_dur = info.as_ref().filter(|i| i.duration > 0.0).map(|i| i.duration);
    let in_fps = info.as_ref().filter(|i| i.fps > 0.0).map(|i| i.fps);
    let timing = TimingSettings {
        mode: if format == Format::Png { OutputMode::Image } else { OutputMode::Video },
        duration: duration.or(in_dur).unwrap_or(4.0).clamp(0.05, 3600.0),
        fps: fps.or(in_fps).unwrap_or(30.0),
        bpm,
    };
    let audio = info.as_ref().filter(|i| i.has_audio).and(input.clone()).map(|p| phthora_media::AudioSource { path: p, start: 0.0, end: None });
    let job = render::ExportJob { input, effects, timing, width: w & !1, height: h & !1, background: [0.0, 0.0, 0.0, 1.0], flip_x: false, format, quality, audio, out };
    let ctx = phthora_gpu::GpuContext::headless()?;
    let progress = render::Progress::default();
    let t0 = std::time::Instant::now();
    let p = render::run(&ctx, &job, &progress)?;
    eprintln!("wrote {} ({} frames, {:.1} s)", p.display(), progress.done.load(std::sync::atomic::Ordering::Relaxed), t0.elapsed().as_secs_f32());
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|a| a == "-h" || a == "--help").unwrap_or(false) {
        println!("{USAGE}");
        return;
    }
    if args.first().map(|a| a == "render").unwrap_or(false) {
        if let Err(e) = cli_render(&args[1..]) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }
    let initial = args.first().map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_title("phthora").with_inner_size([1440.0, 860.0]).with_app_id("phthora").with_drag_and_drop(true),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    if let Err(e) = eframe::run_native("phthora", options, Box::new(move |cc| Ok(Box::new(app::PhthoraApp::new(cc, initial))))) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
