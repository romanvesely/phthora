//! Preview playback of an audio track, looped like the export. While it plays, its sample
//! clock is the preview clock, so picture and sound stay in sync.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::analyzer::SAMPLE_RATE;

/// What is played: mono samples at `SAMPLE_RATE` and how output time maps to them.
#[derive(Clone)]
pub struct Program {
    pub pcm: Arc<Vec<f32>>,
    /// used part of the track (seconds)
    pub start: f64,
    pub span: f64,
    /// output loop length (seconds): the position wraps here
    pub loop_len: f64,
    /// seconds at the end of the used part that blend into the sound before its start
    pub fade: f64,
}

/// Cross-fade at the seam of a loop cut to whole bars, in seconds.
pub const LOOP_FADE: f64 = 0.02;

/// Sample at `u` seconds into the used part (`start`, `span` long) of mono PCM at
/// `SAMPLE_RATE`. Within the last `fade` seconds the part blends (equal power, like the
/// export) into the sound before its start, so the wrap to the start runs on without a click.
pub fn loop_sample(pcm: &[f32], start: f64, span: f64, fade: f64, u: f64) -> f32 {
    let read = |t: f64| {
        if t < 0.0 {
            return 0.0;
        }
        let x = t * SAMPLE_RATE as f64;
        let i = x.floor() as usize;
        let f = (x - x.floor()) as f32;
        let a = pcm.get(i).copied().unwrap_or(0.0);
        let b = pcm.get(i + 1).copied().unwrap_or(a);
        a + (b - a) * f
    };
    let v = read(start + u);
    let fade = fade.min(start).min(span / 2.0);
    if fade > 0.0 && u > span - fade {
        let w = ((u - (span - fade)) / fade) as f32 * std::f32::consts::FRAC_PI_2;
        v * w.cos() + read(start + u - span) * w.sin()
    } else {
        v
    }
}

struct Shared {
    program: Mutex<Option<Program>>,
    /// output time in seconds (f64 bits)
    pos: AtomicU64,
    playing: AtomicBool,
    muted: AtomicBool,
}

pub struct Player {
    shared: Arc<Shared>,
    _stream: cpal::Stream,
    rate: f64,
}

impl Player {
    /// Open the default output device. `None` without a usable device.
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let dev = host.default_output_device().ok_or("no audio output device")?;
        let cfg = dev.default_output_config().map_err(|e| e.to_string())?;
        let channels = cfg.channels() as usize;
        let rate = cfg.sample_rate() as f64;
        let shared = Arc::new(Shared { program: Mutex::new(None), pos: AtomicU64::new(0), playing: AtomicBool::new(false), muted: AtomicBool::new(false) });
        let sh = shared.clone();
        let err = |e: cpal::Error| eprintln!("audio output: {e}");
        let stream = match cfg.sample_format() {
            cpal::SampleFormat::F32 => dev.build_output_stream(cfg.into(), move |out: &mut [f32], _: &_| fill(&sh, out, channels, rate, |v| v), err, None),
            cpal::SampleFormat::I16 => dev.build_output_stream(cfg.into(), move |out: &mut [i16], _: &_| fill(&sh, out, channels, rate, |v| (v * 32767.0) as i16), err, None),
            f => return Err(format!("unsupported output sample format {f}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self { shared, _stream: stream, rate })
    }

    pub fn device_rate(&self) -> f64 {
        self.rate
    }

    pub fn set_program(&self, p: Option<Program>) {
        *self.shared.program.lock().unwrap() = p;
    }

    pub fn has_program(&self) -> bool {
        self.shared.program.lock().unwrap().is_some()
    }

    pub fn set_playing(&self, on: bool) {
        self.shared.playing.store(on, Ordering::Relaxed);
    }

    pub fn set_muted(&self, on: bool) {
        self.shared.muted.store(on, Ordering::Relaxed);
    }

    /// Jump to output time `t`.
    pub fn seek(&self, t: f64) {
        self.shared.pos.store(t.max(0.0).to_bits(), Ordering::Relaxed);
    }

    /// Current output time.
    pub fn position(&self) -> f64 {
        f64::from_bits(self.shared.pos.load(Ordering::Relaxed))
    }
}

fn fill<T: Copy + Default>(sh: &Shared, out: &mut [T], channels: usize, rate: f64, conv: impl Fn(f32) -> T) {
    let prog = sh.program.try_lock().ok().and_then(|g| g.clone());
    let playing = sh.playing.load(Ordering::Relaxed);
    let (Some(p), true) = (prog, playing) else {
        out.fill(T::default());
        return;
    };
    let muted = sh.muted.load(Ordering::Relaxed);
    let mut pos = f64::from_bits(sh.pos.load(Ordering::Relaxed));
    let step = 1.0 / rate;
    for frame in out.chunks_mut(channels) {
        let span = p.span.max(1e-6);
        let v = if muted { 0.0 } else { loop_sample(&p.pcm, p.start, span, p.fade, pos.rem_euclid(span)) };
        frame.fill(conv(v.clamp(-1.0, 1.0)));
        pos += step;
        if p.loop_len > 0.0 && pos >= p.loop_len {
            pos -= p.loop_len;
        }
    }
    sh.pos.store(pos.to_bits(), Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loop_seam_runs_on() {
        let sr = SAMPLE_RATE as f64;
        // 50 Hz sine; the part 1 s .. 1.4649 s ends near a peak
        let pcm: Vec<f32> = (0..3 * SAMPLE_RATE as usize).map(|i| (std::f64::consts::TAU * 50.0 * i as f64 / sr).sin() as f32).collect();
        let (start, span) = (1.0, 20500.0 / sr);
        let at = |fade: f64, k: i64| loop_sample(&pcm, start, span, fade, (k as f64 / sr).rem_euclid(span));
        let jump = |fade: f64| (at(fade, 0) - at(fade, -1)).abs();
        let smooth = (std::f64::consts::TAU * 50.0 / sr) as f32;
        assert!(jump(0.0) > 5.0 * smooth);
        assert!(jump(0.02) <= 1.5 * smooth, "{} vs {smooth}", jump(0.02));
        // outside the fade the part is untouched
        assert_eq!(at(0.02, 100), pcm[44100 + 100]);
        assert_eq!(at(0.02, 10000), at(0.0, 10000));
    }
}
