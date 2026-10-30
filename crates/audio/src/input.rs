//! Live microphone input: the latest samples, resampled to `SAMPLE_RATE`, for live analysis.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::analyzer::{FFT_SIZE, SAMPLE_RATE};

pub struct MicInput {
    ring: Arc<Mutex<VecDeque<f32>>>,
    /// samples since the last `take_new` (at most a second), for the drum detector
    fresh: Arc<Mutex<Vec<f32>>>,
    _stream: cpal::Stream,
    pub name: String,
}

impl MicInput {
    pub fn open() -> Result<Self, String> {
        let host = cpal::default_host();
        let dev = host.default_input_device().ok_or("no audio input device")?;
        let name = dev.description().map(|d| d.name().to_string()).unwrap_or_else(|_| "microphone".into());
        let cfg = dev.default_input_config().map_err(|e| e.to_string())?;
        let channels = cfg.channels() as usize;
        let ratio = SAMPLE_RATE as f64 / cfg.sample_rate() as f64;
        let ring = Arc::new(Mutex::new(VecDeque::with_capacity(FFT_SIZE * 2)));
        let r = ring.clone();
        let fresh = Arc::new(Mutex::new(Vec::new()));
        let fr = fresh.clone();
        let mut phase = 0.0f64;
        let mut prev = 0.0f32;
        // mix down and resample by linear interpolation
        let mut push = move |frames: &mut dyn Iterator<Item = f32>| {
            let mut g = r.lock().unwrap();
            let mut f = fr.lock().unwrap();
            for v in frames {
                while phase < 1.0 {
                    let s = prev + (v - prev) * phase as f32;
                    g.push_back(s);
                    f.push(s);
                    phase += 1.0 / ratio;
                }
                phase -= 1.0;
                prev = v;
            }
            let excess = g.len().saturating_sub(FFT_SIZE * 2);
            g.drain(..excess);
            let excess = f.len().saturating_sub(SAMPLE_RATE as usize);
            f.drain(..excess);
        };
        let err = |e: cpal::Error| eprintln!("audio input: {e}");
        let stream = match cfg.sample_format() {
            cpal::SampleFormat::F32 => dev.build_input_stream(
                cfg.into(),
                move |d: &[f32], _: &_| push(&mut d.chunks(channels).map(|c| c.iter().sum::<f32>() / channels as f32)),
                err,
                None,
            ),
            cpal::SampleFormat::I16 => dev.build_input_stream(
                cfg.into(),
                move |d: &[i16], _: &_| push(&mut d.chunks(channels).map(|c| c.iter().map(|v| *v as f32 / 32768.0).sum::<f32>() / channels as f32)),
                err,
                None,
            ),
            f => return Err(format!("unsupported input sample format {f}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self { ring, fresh, _stream: stream, name })
    }

    /// The samples recorded since the last call.
    pub fn take_new(&self) -> Vec<f32> {
        std::mem::take(&mut *self.fresh.lock().unwrap())
    }

    /// The latest `FFT_SIZE` samples (fewer right after opening).
    pub fn latest(&self) -> Vec<f32> {
        let g = self.ring.lock().unwrap();
        g.iter().skip(g.len().saturating_sub(FFT_SIZE)).copied().collect()
    }
}
