//! Byte frequency data with the semantics of a Web Audio `AnalyserNode` (fftSize 1024,
//! no time smoothing, -100 to -30 dB mapped to 0..255).

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

pub const SAMPLE_RATE: u32 = 44100;
pub const FFT_SIZE: usize = 1024;
pub const BINS: usize = FFT_SIZE / 2;
/// Samples between two analysis snapshots (about 86 per second).
pub const HOP: usize = 512;
const MIN_DB: f32 = -100.0;
const MAX_DB: f32 = -30.0;

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<Complex32>,
    scratch: Vec<Complex32>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Self {
        let fft = FftPlanner::new().plan_fft_forward(FFT_SIZE);
        // Blackman window, alpha 0.16
        let window = (0..FFT_SIZE)
            .map(|i| {
                let x = i as f64 / FFT_SIZE as f64;
                (0.42 - 0.5 * (std::f64::consts::TAU * x).cos() + 0.08 * (2.0 * std::f64::consts::TAU * x).cos()) as f32
            })
            .collect();
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];
        Self { fft, window, buf: vec![Complex32::default(); FFT_SIZE], scratch }
    }

    /// Spectrum of the last `FFT_SIZE` samples of `samples` (missing samples count as silence).
    pub fn analyze(&mut self, samples: &[f32]) -> [u8; BINS] {
        let pad = FFT_SIZE.saturating_sub(samples.len());
        let tail = &samples[samples.len().saturating_sub(FFT_SIZE)..];
        for (i, c) in self.buf.iter_mut().enumerate() {
            let s = if i < pad { 0.0 } else { tail[i - pad] };
            *c = Complex32::new(s * self.window[i], 0.0);
        }
        self.fft.process_with_scratch(&mut self.buf, &mut self.scratch);
        let mut out = [0u8; BINS];
        let scale = 1.0 / FFT_SIZE as f32;
        for (k, o) in out.iter_mut().enumerate() {
            let mag = self.buf[k].norm() * scale;
            let db = if mag > 0.0 { 20.0 * mag.log10() } else { f32::NEG_INFINITY };
            let v = 255.0 * (db - MIN_DB) / (MAX_DB - MIN_DB);
            *o = v.clamp(0.0, 255.0) as u8;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_peaks_at_its_bin() {
        let mut a = Analyzer::new();
        // bin 40 = 40 * 44100 / 1024 Hz
        let f = 40.0 * SAMPLE_RATE as f32 / FFT_SIZE as f32;
        let s: Vec<f32> = (0..2048).map(|i| (std::f32::consts::TAU * f * i as f32 / SAMPLE_RATE as f32).sin() * 0.01).collect();
        let d = a.analyze(&s);
        let peak = d.iter().enumerate().max_by_key(|(_, v)| **v).unwrap().0;
        assert_eq!(peak, 40);
        assert_eq!(d[300], 0);
        assert_eq!(a.analyze(&[]), [0; BINS]);
    }
}
