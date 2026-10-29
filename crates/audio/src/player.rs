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
    let n = p.pcm.len();
    for frame in out.chunks_mut(channels) {
        let tt = p.start + pos.rem_euclid(p.span.max(1e-6));
        let x = tt * SAMPLE_RATE as f64;
        let i = x.floor() as usize;
        let f = (x - x.floor()) as f32;
        let a = p.pcm.get(i).copied().unwrap_or(0.0);
        let b = p.pcm.get((i + 1).min(n.saturating_sub(1))).copied().unwrap_or(0.0);
        let v = if muted { 0.0 } else { a + (b - a) * f };
        frame.fill(conv(v.clamp(-1.0, 1.0)));
        pos += step;
        if p.loop_len > 0.0 && pos >= p.loop_len {
            pos -= p.loop_len;
        }
    }
    sh.pos.store(pos.to_bits(), Ordering::Relaxed);
}
