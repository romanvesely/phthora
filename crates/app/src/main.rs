//! phthora: real-time GPU effect mixer for images, video and GIFs.

mod app;
mod audio_ui;
mod batch;
mod batch_ui;
mod icons;
mod params_ui;
mod pipeline;
mod render;
mod store;
mod video;

use batch::{AudioChoice, BackendPref, Overrides, Shared};
use phthora_core::{Direction, EffectInstance, Playback};
use phthora_media::{Format, Quality};
use std::path::PathBuf;

const USAGE: &str = "usage:
  phthora [FILE]                     open the editor (optionally with an image or video)
  phthora render [INPUT] [options]   render without a window
  phthora render INPUT... --out-dir DIR [options]
                                     render several inputs one after the other, see batch below
options for render:
  --preset FILE        effect chain (.phthora.json)
  --effect ID,...      add effects with default settings (see --list)
  --out FILE           output file, the format follows the name:
                         .mp4 .webm .gif .apng      video
                         .webp                      animated for video input, --duration or --fps, else a still
                         .png .jpg                  one still image
                         DIR/ or name-%05d.png      PNG sequence (a folder gets <folder>-00001.png ...)
                       .webm, .gif, .webp, .apng, .png and sequences keep transparency (--background transparent)
  --duration SEC       output length (default: one pass through the input, 4 s for stills)
  --fps N              frame rate (default: input rate or 30; GIF at most 50)
  --size WxH           output size, even numbers up to 4096 (default: input size)
  --quality Q          ultra | high | medium | low (MP4, WebM, WebP, JPG; GIF: 256/256/128/64 colors)
  --bpm N|auto         sync effect loops to a tempo (auto: detected in the audio)
  --bars N             cut the output to N bars of the tempo; the audio loops without a click
  --loop-start bar:N|SEC  where in the audio the loop starts (bar 0 = the first; needs --bars)
  --background COLOR   #rrggbb, #rrggbbaa or transparent (default: black)
  --flip               mirror the input horizontally
  --trim START END     use only this part of the input (seconds; END may be 'end')
  --speed X            input playback speed, 0.1 to 4
  --reverse            play the input backwards
  --pingpong           play the input forwards, then backwards
  --audio FILE|none    audio track (default: the input's own audio)
  --audio-start SEC    skip the start of the audio track
  --audio-gain G       level of audio modulators, 0 to 5 (default 1)
  --audio-smoothing S  smoothing of audio modulators, 0 to 1 (default 0.5)
  --no-reduce-noise    do not push quiet audio levels down
  --audio-sensitivity S  drum hit picking, 0.5 (clear hits) to 2 (default 1)
  --backend B          auto | software | vaapi (MP4 encoder; auto uses VAAPI if it works)
  --list               list effect ids
batch (several inputs, or --out-dir with one):
  --out-dir DIR        folder for the results (created if missing), instead of --out. Every input is
                       written as <name>-phthora-<date>-<time>.<ext>; no existing file is overwritten.
  --format EXT         one format for all: mp4 webm gif webp apng png jpg seq (a PNG sequence folder).
                       Default: each input's own container where phthora can write it (mp4, webm, gif,
                       webp, png, jpg), else mp4 for video and png for stills.
  Size, length and frame rate come from each input; --size, --duration and --fps then hold for all.
  An input that fails does not stop the others; the failures are listed at the end and the exit
  status is 1. Progress per file goes to stderr. Use shell globs for many files: phthora render *.mp4 --out-dir out";

fn arg_value(args: &[String], i: &mut usize) -> Result<String, String> {
    *i += 1;
    args.get(*i).cloned().ok_or_else(|| format!("missing value for {}", args[*i - 1]))
}

fn parse_num(args: &[String], i: &mut usize) -> Result<f64, String> {
    let name = args[*i].clone();
    let v = arg_value(args, i)?;
    v.parse::<f64>().ok().filter(|x| x.is_finite()).ok_or_else(|| format!("bad value for {name}: {v}"))
}

/// `#rrggbb`, `#rrggbbaa` or `transparent` to straight RGBA 0..1.
fn parse_background(s: &str) -> Result<[f32; 4], String> {
    if s == "transparent" {
        return Ok([0.0; 4]);
    }
    let h = s.trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).map_err(|_| format!("bad color {s}"))?;
    let c = |shift: u32| ((n >> shift) & 0xff) as f32 / 255.0;
    match h.len() {
        6 => Ok([c(16), c(8), c(0), 1.0]),
        8 => Ok([c(24), c(16), c(8), c(0)]),
        _ => Err(format!("bad color {s}, use #rrggbb or #rrggbbaa")),
    }
}

/// Output format from the `--out` name: a folder (trailing `/`) or a `%05d` name is a PNG
/// sequence; `.webp` is moving when `animated` (video input, `--duration` or `--fps`).
fn output_format(out: &std::path::Path, animated: bool) -> Result<Format, String> {
    if out.to_string_lossy().ends_with('/') {
        return Ok(Format::PngSeq);
    }
    Format::from_path(out, animated).ok_or_else(|| "output must end in .mp4, .webm, .gif, .webp, .apng, .png or .jpg, be a folder (trailing /) or a name like frame-%05d.png".into())
}

fn cli_render(args: &[String]) -> Result<(), String> {
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut out_dir: Option<PathBuf> = None;
    let mut format_arg: Option<Format> = None;
    let mut preset: Option<PathBuf> = None;
    let mut effects_arg: Vec<String> = Vec::new();
    let mut out: Option<PathBuf> = None;
    let mut duration: Option<f64> = None;
    let mut fps: Option<f64> = None;
    let mut size: Option<(u32, u32)> = None;
    let mut quality = Quality::High;
    let mut bpm = None;
    let mut bpm_auto = false;
    let mut bars: Option<f64> = None;
    let mut loop_start = None;
    let mut background = [0.0, 0.0, 0.0, 1.0];
    let mut flip_x = false;
    let mut playback = Playback::default();
    let mut audio_arg: Option<String> = None;
    let mut audio_start = 0.0;
    let mut backend = "auto".to_string();
    let mut audio_settings = phthora_audio::AudioSettings::default();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--preset" => preset = Some(arg_value(args, &mut i)?.into()),
            "--effect" => effects_arg.extend(arg_value(args, &mut i)?.split(',').map(|s| s.trim().to_string())),
            "--out" => out = Some(arg_value(args, &mut i)?.into()),
            "--out-dir" => out_dir = Some(arg_value(args, &mut i)?.into()),
            "--format" => {
                let v = arg_value(args, &mut i)?;
                format_arg = Some(batch::parse_format(&v).ok_or_else(|| format!("unknown --format {v}, use mp4, webm, gif, webp, apng, png, jpg or seq"))?);
            }
            "--duration" => duration = Some(parse_num(args, &mut i)?),
            "--fps" => fps = Some(parse_num(args, &mut i)?),
            "--bpm" if args.get(i + 1).map(String::as_str) == Some("auto") => {
                bpm_auto = true;
                i += 1;
            }
            "--bpm" => bpm = Some(parse_num(args, &mut i)?),
            "--bars" => bars = Some(parse_num(args, &mut i)?),
            "--loop-start" => {
                let v = arg_value(args, &mut i)?;
                loop_start = Some(match v.strip_prefix("bar:") {
                    Some(n) => batch::LoopStart::Bar(n.parse().map_err(|_| format!("bad --loop-start {v}"))?),
                    None => batch::LoopStart::Seconds(v.parse().map_err(|_| format!("bad --loop-start {v}, use bar:N or seconds"))?),
                });
            }
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
            "--background" => background = parse_background(&arg_value(args, &mut i)?)?,
            "--flip" => flip_x = true,
            "--trim" => {
                playback.trim_start = parse_num(args, &mut i)?;
                let e = arg_value(args, &mut i)?;
                playback.trim_end = if e == "end" { None } else { Some(e.parse().map_err(|_| format!("bad trim end {e}"))?) };
            }
            "--speed" => playback.speed = parse_num(args, &mut i)?,
            "--reverse" => playback.direction = Direction::Reverse,
            "--pingpong" => playback.direction = Direction::PingPong,
            "--audio" => audio_arg = Some(arg_value(args, &mut i)?),
            "--audio-start" => audio_start = parse_num(args, &mut i)?,
            "--backend" => backend = arg_value(args, &mut i)?,
            "--audio-gain" => audio_settings.gain = parse_num(args, &mut i)? as f32,
            "--audio-smoothing" => audio_settings.smoothing = parse_num(args, &mut i)? as f32,
            "--no-reduce-noise" => audio_settings.reduce_noise = false,
            "--audio-sensitivity" => audio_settings.sensitivity = parse_num(args, &mut i)? as f32,
            "--list" => {
                use std::io::Write;
                let mut out = std::io::stdout().lock();
                // grouped by category, in the order of the editor's effect list
                let cats = phthora_core::registry::categories();
                let mut list: Vec<_> = phthora_core::registry::available().collect();
                list.sort_by_key(|d| cats.iter().position(|c| *c == d.category));
                let w = list.iter().map(|d| d.id.len()).max().unwrap_or(0);
                for d in list {
                    // stop quietly when the reader goes away (e.g. piped into head)
                    if writeln!(out, "{:w$} {:24} {}", d.id, d.category, d.name).is_err() {
                        break;
                    }
                }
                return Ok(());
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            _ if a.starts_with("--") => return Err(format!("unknown option {a}\n{USAGE}")),
            _ => inputs.push(a.into()),
        }
        i += 1;
    }
    let batch_mode = out_dir.is_some() || inputs.len() > 1;
    if batch_mode {
        if out.is_some() {
            return Err("--out names one file: with several inputs or --out-dir use --out-dir only".into());
        }
        if out_dir.is_none() {
            return Err("several inputs need --out-dir DIR for the results".into());
        }
        if inputs.is_empty() {
            return Err("--out-dir needs at least one input".into());
        }
    } else if out.is_none() {
        return Err("--out is required".into());
    } else if format_arg.is_some() {
        return Err("--format is for --out-dir; with --out the format follows the file name".into());
    }
    if !(0.0..=5.0).contains(&audio_settings.gain) || !(0.0..=1.0).contains(&audio_settings.smoothing) {
        return Err("--audio-gain must be 0 to 5, --audio-smoothing 0 to 1".into());
    }
    if !(0.5..=2.0).contains(&audio_settings.sensitivity) {
        return Err("--audio-sensitivity must be 0.5 to 2".into());
    }
    if bars.is_some_and(|b| !(1.0..=256.0).contains(&b) || b.fract() != 0.0) {
        return Err("--bars must be a whole number from 1 to 256".into());
    }
    if bars.is_some() && bpm.is_none() && !bpm_auto {
        return Err("--bars needs --bpm N or --bpm auto".into());
    }
    if loop_start.is_some() && bars.is_none() {
        return Err("--loop-start needs --bars".into());
    }
    if bars.is_some() && duration.is_some() {
        return Err("--bars sets the length: leave out --duration".into());
    }
    if !(Playback::MIN_SPEED..=Playback::MAX_SPEED).contains(&playback.speed) {
        return Err(format!("--speed must be between {} and {}", Playback::MIN_SPEED, Playback::MAX_SPEED));
    }
    if let Some(f) = fps
        && !(1.0..=120.0).contains(&f) {
            return Err("--fps must be between 1 and 120".into());
        }
    if let Some(d) = duration
        && !(0.05..=3600.0).contains(&d) {
            return Err("--duration must be between 0.05 and 3600 seconds".into());
        }
    if let Some((w, h)) = size
        && (!(16..=4096).contains(&w) || !(16..=4096).contains(&h)) {
            return Err("--size must be between 16x16 and 4096x4096".into());
        }
    if !matches!(backend.as_str(), "auto" | "software" | "vaapi") {
        return Err(format!("unknown backend {backend}"));
    }
    let mut chain = phthora_core::Chain::default();
    if let Some(p) = &preset {
        let s = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let (fx, warnings) = phthora_core::preset::from_json(&s).map_err(|e| e.to_string())?;
        for w in warnings {
            eprintln!("preset: {w}");
        }
        for m in phthora_core::preset::missing_files(&fx) {
            eprintln!("preset: file not found: {m}");
        }
        chain = fx;
    }
    for id in &effects_arg {
        let mut e = EffectInstance::by_id(id).filter(|e| e.desc.available()).ok_or_else(|| format!("unknown effect {id} (see --list)"))?;
        e.enabled = true;
        chain.effects.push(e);
    }
    let shared = Shared {
        chain,
        playback,
        bpm,
        bpm_auto,
        bars: bars.map(|b| b as u32),
        loop_start,
        background,
        flip_x,
        quality,
        backend: match backend.as_str() {
            "software" => BackendPref::Software,
            "vaapi" => BackendPref::Vaapi,
            _ => BackendPref::Auto,
        },
        audio: match audio_arg.as_deref() {
            Some("none") => AudioChoice::None,
            Some(f) => AudioChoice::File(f.into()),
            None => AudioChoice::Input,
        },
        audio_start,
        audio_settings,
        midi: Default::default(),
    };
    let ov = Overrides { size, duration, fps, format: format_arg };
    if batch_mode {
        return cli_batch(&inputs, &out_dir.expect("checked above"), &shared, &ov);
    }
    let (input, out) = (inputs.first().cloned(), out.expect("checked above"));
    let info = match &input {
        Some(p) => Some(phthora_media::probe(p).map_err(|e| e.to_string())?),
        None => None,
    };
    let moving_input = info.as_ref().is_some_and(|i| i.kind != phthora_media::MediaKind::Image);
    let format = output_format(&out, moving_input || duration.is_some() || fps.is_some())?;
    let built = batch::build_job(&shared, &ov, input.as_deref(), info.as_ref(), format, out)?;
    if let Some(n) = &built.note {
        eprintln!("note: {n}");
    }
    let ctx = phthora_gpu::GpuContext::headless()?;
    let progress = render::Progress::default();
    let t0 = std::time::Instant::now();
    let p = render::run(&ctx, &built.job, &progress)?;
    eprintln!("wrote {} ({} frames, {:.1} s)", p.display(), progress.done.load(std::sync::atomic::Ordering::Relaxed), t0.elapsed().as_secs_f32());
    Ok(())
}

/// `phthora render A B C --out-dir DIR`: every input with the same chain, progress per file on stderr.
fn cli_batch(inputs: &[PathBuf], out_dir: &std::path::Path, shared: &Shared, ov: &Overrides) -> Result<(), String> {
    use batch::Event;
    let ctx = phthora_gpu::GpuContext::headless()?;
    let progress = batch::BatchProgress::default();
    let name = |p: &std::path::Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let report = batch::run_batch(&ctx, inputs, out_dir, shared, ov, &progress, |e| match e {
        Event::Start { index, total, input } => eprintln!("[{index}/{total}] {} ...", input.display()),
        Event::Note(n) => eprintln!("note: {n}"),
        Event::Done { index, total, out, frames, secs } => eprintln!("[{index}/{total}] wrote {} ({frames} frames, {secs:.1} s)", out.display()),
        Event::Failed { index, total, input, error } => eprintln!("[{index}/{total}] FAILED {}: {}", name(input), batch::short_error(input, error)),
    });
    eprintln!("{} of {} exported to {}", report.done.len(), inputs.len(), out_dir.display());
    if report.failed.is_empty() {
        return Ok(());
    }
    let list: Vec<String> = report.failed.iter().map(|f| format!("  {}: {}", f.input.display(), batch::short_error(&f.input, &f.error))).collect();
    Err(format!("{} of {} inputs failed:\n{}", report.failed.len(), inputs.len(), list.join("\n")))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|a| a == "-h" || a == "--help").unwrap_or(false) {
        println!("{USAGE}");
        return;
    }
    if args.first().map(|a| a == "-V" || a == "--version").unwrap_or(false) {
        println!("phthora {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.first().map(|a| a == "render").unwrap_or(false) {
        if let Err(e) = cli_render(&args[1..]) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }
    let _log = init_log();
    tracing::info!("phthora {} starting", env!("CARGO_PKG_VERSION"));
    let initial = args.first().map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_title("phthora").with_inner_size([1440.0, 860.0]).with_app_id("phthora").with_drag_and_drop(true),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    if let Err(e) = eframe::run_native("phthora", options, Box::new(move |cc| Ok(Box::new(app::PhthoraApp::new(cc, initial))))) {
        tracing::error!("cannot start: {e}");
        eprintln!("error: {e}");
        // most often the GPU: say so in a window, the terminal may not be visible
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("phthora cannot start")
            .set_description(format!("{e}\n\nphthora needs a GPU with Vulkan, Metal or DirectX 12 support. Details are in {}.", store::log_dir().display()))
            .show();
        std::process::exit(1);
    }
}

/// Log to `data_dir/logs/phthora.YYYY-MM-DD.log`, one file per day, the last five kept.
fn init_log() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("phthora")
        .filename_suffix("log")
        .max_log_files(5)
        .build(store::log_dir())
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt().with_writer(writer).with_ansi(false).with_target(false).init();
    Some(guard)
}
