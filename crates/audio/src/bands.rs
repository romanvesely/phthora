//! Band levels and the log spectrum derived from byte frequency data, with gain, noise
//! reduction (an S-curve) and smoothing over frames.

use crate::analyzer::{BINS, SAMPLE_RATE};
use serde::{Deserialize, Serialize};

const MAX_FREQ: f32 = SAMPLE_RATE as f32 / 2.0;
const SIGMOID_STRENGTH: f32 = 12.0;
const SIGMOID_MIDPOINT: f32 = 0.65;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BandDef {
    pub lo_hz: f32,
    pub hi_hz: f32,
    pub gain: f32,
}

/// all, low, mid, hi
pub const DEFAULT_BANDS: [BandDef; 4] = [
    BandDef { lo_hz: 0.0, hi_hz: 4000.0, gain: 1.0 },
    BandDef { lo_hz: 40.0, hi_hz: 140.0, gain: 0.8 },
    BandDef { lo_hz: 400.0, hi_hz: 1000.0, gain: 1.2 },
    BandDef { lo_hz: 1000.0, hi_hz: 4000.0, gain: 1.4 },
];

fn bin_for(hz: f32) -> usize {
    ((hz.clamp(0.0, MAX_FREQ) / MAX_FREQ) * (BINS - 1) as f32).round() as usize
}

impl BandDef {
    /// Raw level of the band (mean byte value / 256 times the band gain).
    pub fn volume(&self, data: &[u8; BINS]) -> f32 {
        let (a, b) = (bin_for(self.lo_hz), bin_for(self.hi_hz.max(self.lo_hz)));
        let total: u32 = data[a..=b].iter().map(|v| *v as u32).sum();
        // divides by the span, not the bin count, as the reference does
        total as f32 / (b - a).max(1) as f32 / 256.0 * self.gain
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    /// 0 ..= 5
    pub gain: f32,
    /// 0 (none) ..= 1 (frozen)
    pub smoothing: f32,
    pub reduce_noise: bool,
    /// extra bands after all/low/mid/hi
    pub custom: Vec<BandDef>,
    /// drum hit picking: 0.5 (only clear hits) ..= 2 (also doubtful ones)
    pub sensitivity: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { gain: 1.0, smoothing: 0.5, reduce_noise: true, custom: Vec::new(), sensitivity: 1.0 }
    }
}

fn sigmoid(v: f32) -> f32 {
    let s = |x: f32| 1.0 / (1.0 + (-SIGMOID_STRENGTH * (x - SIGMOID_MIDPOINT)).exp());
    (s(v) - s(0.0)) / (s(1.0) - s(0.0))
}

/// Smoothed levels for one output frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Levels {
    /// all, low, mid, hi, then the custom bands; 0..1
    pub bands: Vec<f32>,
    /// log-frequency spectrum, 0..1 (audio visualizer)
    pub spectrum: Vec<f32>,
}

impl Default for Levels {
    fn default() -> Self {
        Self { bands: vec![0.0; 4], spectrum: vec![0.0; BINS] }
    }
}

/// Follows the analysis frame by frame. Step it exactly once per output frame (preview and
/// export alike) and reset it when playback jumps.
#[derive(Default)]
pub struct Follower {
    levels: Levels,
}

impl Follower {
    pub fn reset(&mut self) {
        self.levels = Levels::default();
    }

    pub fn levels(&self) -> &Levels {
        &self.levels
    }

    /// Advance by one frame with spectrum `data` (`None`: silence).
    pub fn step(&mut self, data: Option<&[u8; BINS]>, s: &AudioSettings) -> &Levels {
        let k = 1.0 - s.smoothing.clamp(0.0, 1.0);
        let gain = s.gain.clamp(0.0, 5.0);
        let n = 4 + s.custom.len();
        self.levels.bands.resize(n, 0.0);
        let shape = |v: f32| {
            let v = (v * gain).min(1.0);
            if s.reduce_noise { sigmoid(v) } else { v }
        };
        for (i, b) in DEFAULT_BANDS.iter().chain(&s.custom).enumerate() {
            let v = data.map(|d| shape(b.volume(d))).unwrap_or(0.0);
            self.levels.bands[i] += (v - self.levels.bands[i]) * k;
        }
        let log_scale = (BINS as f32).ln() / BINS as f32;
        const HI_BOOST: f32 = 0.3;
        for i in 0..BINS {
            let v = match data {
                Some(d) => {
                    let li = (i as f32 * log_scale).exp();
                    let lo = (li.floor() as usize).min(BINS - 1);
                    let hi = (li.ceil() as usize).min(BINS - 1);
                    let (lv, hv) = (d[lo] as f32, d[hi] as f32);
                    let w = if lo == hi { 0.0 } else { (li - lo as f32) / (hi - lo) as f32 };
                    let boost = 1.0 + (i as f32 / BINS as f32 - 0.5) * 2.0 * HI_BOOST;
                    let v = ((lv + (hv - lv) * w) * boost).clamp(0.0, 255.0) / 255.0;
                    shape(v)
                }
                None => 0.0,
            };
            self.levels.spectrum[i] += (v - self.levels.spectrum[i]) * k;
        }
        &self.levels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_volume_and_shaping() {
        let mut d = [0u8; BINS];
        // low band: bins 1..=3 at 40..140 Hz
        assert_eq!((bin_for(40.0), bin_for(140.0)), (1, 3));
        d[1..=3].fill(200);
        let v = DEFAULT_BANDS[1].volume(&d);
        assert!((v - 600.0 / 2.0 / 256.0 * 0.8).abs() < 1e-6);
        assert!(sigmoid(0.0).abs() < 1e-6 && (sigmoid(1.0) - 1.0).abs() < 1e-6);
        assert!(sigmoid(0.3) < 0.3 && sigmoid(0.9) > 0.9);
        let s = AudioSettings { reduce_noise: false, smoothing: 0.0, ..Default::default() };
        let mut f = Follower::default();
        let l = f.step(Some(&d), &s).clone();
        assert!((l.bands[1] - v.min(1.0)).abs() < 1e-6);
        // smoothing: half way per frame
        let s = AudioSettings { reduce_noise: false, smoothing: 0.5, ..Default::default() };
        f.reset();
        f.step(Some(&d), &s);
        assert!((f.levels().bands[1] - v / 2.0).abs() < 1e-6);
        f.step(None, &s);
        assert!((f.levels().bands[1] - v / 4.0).abs() < 1e-6);
        // custom band
        let s = AudioSettings { custom: vec![BandDef { lo_hz: 40.0, hi_hz: 140.0, gain: 1.0 }], smoothing: 0.0, reduce_noise: false, ..Default::default() };
        f.reset();
        assert_eq!(f.step(Some(&d), &s).bands.len(), 5);
    }
}
