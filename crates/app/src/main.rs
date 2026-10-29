//! phthora: real-time GPU effect mixer for images, video and GIFs.

mod app;
mod audio_ui;
mod params_ui;
mod pipeline;
mod render;
mod store;
mod video;

use phthora_core::{ClipTiming, Direction, EffectInstance, OutputMode, Playback, TimingSettings};
use phthora_media::{AudioSource, Backend, Format, Quality};
use std::path::PathBuf;

const USAGE: &str = "usage:
  phthora [FILE]                     open the editor (optionally with an image or video)
  phthora render [INPUT] [options]   render without a window
options for render:
  --preset FILE        effect chain (.phthora.json)
  --effect ID,...      add effects with default settings (see --list)
  --out FILE           output file (format from extension: mp4, webm, gif, png)
  --duration SEC       output length (default: one pass through the input, 4 s for stills)
  --fps N              frame rate (default: input rate or 30)
  --size WxH           output size, even numbers up to 4096 (default: input size)
  --quality Q          ultra | high | medium | low
  --bpm N              sync effect loops to a tempo
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
  --backend B          auto | software | vaapi (MP4 encoder; auto uses VAAPI if it works)
  --list               list effect ids";

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
            "--duration" => duration = Some(parse_num(args, &mut i)?),
            "--fps" => fps = Some(parse_num(args, &mut i)?),
            "--bpm" => bpm = Some(parse_num(args, &mut i)?),
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
    if !(0.0..=5.0).contains(&audio_settings.gain) || !(0.0..=1.0).contains(&audio_settings.smoothing) {
        return Err("--audio-gain must be 0 to 5, --audio-smoothing 0 to 1".into());
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
    let backend = match (backend.as_str(), format) {
        (_, f) if f != Format::Mp4 => Backend::Software,
        ("software", _) => Backend::Software,
        ("vaapi", _) => {
            if !phthora_media::vaapi_available() {
                return Err("--backend vaapi: no working VAAPI H.264 encoder found".into());
            }
            Backend::Vaapi
        }
        ("auto", _) => if phthora_media::vaapi_available() { Backend::Vaapi } else { Backend::Software },
        (b, _) => return Err(format!("unknown backend {b}")),
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
    let video = info.as_ref().filter(|i| i.kind != phthora_media::MediaKind::Image && i.duration > 0.0);
    let clip = video.map(|i| ClipTiming { fps: if i.fps > 0.0 { i.fps } else { 30.0 }, frames: ((i.duration * if i.fps > 0.0 { i.fps } else { 30.0 }).round() as u64).max(1) });
    let in_dur = clip.map(|c| playback.natural_duration(c));
    let in_fps = video.filter(|i| i.fps > 0.0).map(|i| i.fps.round().clamp(1.0, 120.0));
    let timing = TimingSettings {
        mode: if format == Format::Png { OutputMode::Image } else { OutputMode::Video },
        duration: duration.or(in_dur).unwrap_or(4.0).clamp(0.05, 3600.0),
        fps: fps.or(in_fps).unwrap_or(30.0),
        bpm,
    };
    // the input's own audio follows the trim; reversed or sped-up input has no matching audio
    let audio = match audio_arg.as_deref() {
        Some("none") => None,
        Some(f) => Some(AudioSource { path: f.into(), start: audio_start, end: None }),
        None if playback.speed == 1.0 && playback.direction == Direction::Forward => video
            .filter(|i| i.has_audio)
            .and(input.clone())
            .map(|p| AudioSource { path: p, start: playback.trim_start + audio_start, end: playback.trim_end }),
        None => None,
    };
    let audio_track = match &audio {
        Some(a) => {
            let mut t = phthora_audio::AudioTrack::load(&a.path).map_err(|e| format!("audio: {e}"))?;
            t.start = a.start;
            t.end = a.end;
            Some(t)
        }
        None => None,
    };
    let job = render::ExportJob { input, playback, effects, timing, width: w & !1, height: h & !1, background, flip_x, format, quality, backend, audio, audio_track, audio_settings, midi: Default::default(), out };
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
