//! Tempo of a track: beats per minute, where the beats fall and which of them starts a bar,
//! from the same band levels as the drum hits.

use crate::onset::{Bands, FRAME_RATE};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tempo {
    pub bpm: f64,
    /// time of the first beat in seconds (0 ≤ first_beat < one beat)
    pub first_beat: f64,
    /// which of the beats 0..4 after `first_beat` starts a bar
    pub downbeat: u8,
    /// 0 (a guess) ..= 1 (a clear, steady beat)
    pub confidence: f64,
}

impl Tempo {
    pub fn beat_len(&self) -> f64 {
        60.0 / self.bpm
    }

    pub fn bar_len(&self) -> f64 {
        4.0 * self.beat_len()
    }

    /// Start of bar `n` (0 = the first bar that starts in the track) in seconds.
    pub fn bar_start(&self, n: u32) -> f64 {
        self.first_beat + (self.downbeat as f64 + 4.0 * n as f64) * self.beat_len()
    }
}

const MIN_BPM: f64 = 60.0;
const MAX_BPM: f64 = 200.0;
/// Shorter tracks give no reliable tempo.
const MIN_SECONDS: f64 = 6.0;
/// Only the first minutes are looked at: enough for a steady tempo, and quick.
const MAX_SECONDS: f64 = 150.0;
/// Tempo most dance music is near; of two tempi an octave apart the one closer to it wins.
const PRIOR_BPM: f64 = 120.0;
/// Width of that preference in octaves.
const PRIOR_WIDTH: f64 = 0.6;

const KICK_WEIGHTS: [f32; 3] = [1.0, 0.1, 0.05];
const ALL_WEIGHTS: [f32; 3] = [1.0, 1.0, 0.5];
const SYNCOPATION: f64 = 0.8;

/// Onset strength per frame: the rises of the drum bands above their local mean. The low end
/// (the kick) counts most: hi-hats between the beats would otherwise pull the grid onto the
/// off-beats, or to a tempo 1.5 or 2 times off.
fn onset_curve(b: &Bands, n: usize, w: [f32; 3]) -> Vec<f32> {
    let rise = |v: &[f32], k: usize| if k == 0 { 0.0 } else { (v[k] - v[k - 1]).max(0.0) };
    let raw: Vec<f32> = (0..n).map(|k| w[0] * rise(&b.low, k) + w[1] * rise(&b.mid, k) + w[2] * rise(&b.high, k)).collect();
    // subtract a moving mean of a quarter second: steady noise is no beat
    const HALF: usize = 21;
    let mut pre = vec![0.0f64; n + 1];
    for (k, v) in raw.iter().enumerate() {
        pre[k + 1] = pre[k] + *v as f64;
    }
    (0..n)
        .map(|k| {
            let (a, z) = (k.saturating_sub(HALF), (k + HALF + 1).min(n));
            let mean = ((pre[z] - pre[a]) / (z - a) as f64) as f32;
            (raw[k] - mean).max(0.0)
        })
        .collect()
}

/// Value at a fractional frame, linear between neighbours, 0 outside.
fn at(o: &[f32], x: f64) -> f32 {
    let i = x.floor() as usize;
    let f = (x - x.floor()) as f32;
    let a = o.get(i).copied().unwrap_or(0.0);
    let b = o.get(i + 1).copied().unwrap_or(0.0);
    a + (b - a) * f
}

/// Mean product of the curve with itself `lag` frames later.
fn autocorr(o: &[f32], lag: f64) -> f64 {
    let n = o.len() as f64 - lag - 1.0;
    if n < 1.0 {
        return 0.0;
    }
    let m = n as usize;
    (0..m).map(|k| (o[k] * at(o, k as f64 + lag)) as f64).sum::<f64>() / m as f64
}

fn prior(bpm: f64) -> f64 {
    let x = (bpm / PRIOR_BPM).log2() / PRIOR_WIDTH;
    (-0.5 * x * x).exp()
}

/// How well beats every `period` frames, and multiples of them, repeat in the curve.
fn comb(o: &[f32], period: f64) -> f64 {
    autocorr(o, period) + 0.5 * autocorr(o, 2.0 * period) + 0.25 * autocorr(o, 4.0 * period)
}

/// Best phase (in frames, 0 ≤ phase < period) of a beat grid and how much onset it collects.
fn best_phase(g: &[f32], period: f64) -> (f64, f64) {
    let beats = ((g.len() as f64 - 1.0) / period).floor() as usize;
    let mut best = (0.0, f64::NEG_INFINITY);
    let steps = (period * 2.0).ceil() as usize;
    for s in 0..steps {
        let phase = s as f64 * 0.5;
        let sum: f64 = (0..beats).map(|j| at(g, phase + j as f64 * period) as f64).sum();
        if sum > best.1 {
            best = (phase, sum);
        }
    }
    best
}

/// Detect the tempo, `None` for short tracks or without any beat.
pub fn detect(b: &Bands) -> Option<Tempo> {
    let n = b.low.len().min((MAX_SECONDS * FRAME_RATE) as usize);
    if (n as f64) < MIN_SECONDS * FRAME_RATE {
        return None;
    }
    let mut o = onset_curve(b, n, KICK_WEIGHTS);
    if o.iter().all(|v| *v <= 0.0) {
        return None;
    }
    let period = |bpm: f64| 60.0 * FRAME_RATE / bpm;
    // coarse: every half BPM, weighted towards the usual tempi
    let coarse: Vec<(f64, f64)> = (0..=((MAX_BPM - MIN_BPM) * 2.0) as usize)
        .map(|i| {
            let bpm = MIN_BPM + i as f64 * 0.5;
            (bpm, comb(&o, period(bpm)) * prior(bpm))
        })
        .collect();
    let &(mut bpm0, top) = coarse.iter().max_by(|a, b| a.1.total_cmp(&b.1))?;
    if top <= 0.0 {
        return None;
    }
    // a syncopated kick (breakbeats) repeats every 1.5 beats; snares and hats then show the
    // real beat, 1.5 times faster
    if bpm0 * 1.5 <= MAX_BPM {
        let all = onset_curve(b, n, ALL_WEIGHTS);
        if comb(&all, period(bpm0 * 1.5)) >= SYNCOPATION * comb(&all, period(bpm0)) {
            bpm0 *= 1.5;
            // the kick is off the beat half the time: snares and hats place the grid
            o = all;
        }
    }
    let mean = coarse.iter().map(|c| c.1).sum::<f64>() / coarse.len() as f64;
    // fine: a beat grid over the whole part, allowing a frame of jitter, pins the tempo down
    let g: Vec<f32> = (0..n).map(|k| o[k.saturating_sub(1)].max(o[k]).max(o[(k + 1).min(n - 1)])).collect();
    let mut best = (bpm0, 0.0, f64::NEG_INFINITY);
    for i in 0..=120 {
        let bpm = bpm0 - 0.6 + i as f64 * 0.01;
        let (phase, sum) = best_phase(&g, period(bpm));
        // the sum grows with the number of beats: compare per beat
        let per_beat = sum * period(bpm);
        if per_beat > best.2 {
            best = (bpm, phase, per_beat);
        }
    }
    let (bpm, phase, _) = best;
    let p = period(bpm);
    // the bar starts on the beat with the loudest low end (the kick) over the track
    let beats = ((n as f64 - 1.0 - phase) / p).floor() as usize;
    let mut per = [0.0f64; 4];
    for j in 0..beats {
        let k = phase + j as f64 * p;
        per[j % 4] += (0..10).map(|d| at(&b.low, k + d as f64)).fold(f32::NEG_INFINITY, f32::max) as f64;
    }
    let downbeat = (0..4).max_by(|&a, &b| per[a].total_cmp(&per[b])).unwrap_or(0) as u8;
    Some(Tempo { bpm: (bpm * 100.0).round() / 100.0, first_beat: phase / FRAME_RATE, downbeat, confidence: (1.0 - mean / top).clamp(0.0, 1.0) })
}
