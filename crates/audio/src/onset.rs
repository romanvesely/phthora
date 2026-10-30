//! Drum hits: kick drum, snare or clap, and hi-hat.
//!
//! The track is cut into frames every `STEP` samples. Per frame there are band levels in dB
//! (a long window for the low end, a short one for precise timing above) and the flatness of
//! the mid and high bands (noise is flat, a synth line is not). Candidates are sudden rises of
//! a drum's band; a small linear model on the shape of the rise decides whether it is the drum.
//! The model was fitted on rendered electronic tracks with known hit times (four-on-the-floor
//! techno, house, acid, minimal, trap, breaks).

use crate::analyzer::SAMPLE_RATE;
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// kick, snare (or clap), hi-hat
pub const DRUMS: usize = 3;
pub const DRUM_NAMES: [&str; DRUMS] = ["kick", "snare", "hi-hat"];
/// Samples between two frames (about 172 frames per second).
pub const STEP: usize = 256;
pub const FRAME_RATE: f64 = SAMPLE_RATE as f64 / STEP as f64;
const LONG: usize = 2048;
const SHORT: usize = 512;
/// Levels below the loudest one minus this do not count (silence, dither).
const FLOOR_DB: f32 = 60.0;
/// Half width of the window the candidate threshold follows, in frames.
const MEAN_HALF: usize = 43;
const THRESHOLD: f32 = 1.2;
const MIN_RISE: f32 = 0.3;
/// Shortest time between two hits of the same drum, in seconds.
const MIN_GAP: [f64; DRUMS] = [0.09, 0.07, 0.05];
/// Frames needed after a candidate for its features.
const AFTER: [usize; DRUMS] = [8, 16, 12];

/// Linear models (weights per feature, bias); a candidate is a hit when the sum is above 0.
const KICK_MODEL: ([f32; 4], f32) = ([0.29466, 1.38014, 0.20908, 0.10776], -3.13731);
const SNARE_MODEL: ([f32; 6], f32) = ([-0.10123, 0.00025, 1.67809, -0.04323, 0.30965, 0.08346], -13.86092);
const HAT_MODEL: ([f32; 6], f32) = ([-0.14205, 0.06596, 0.60856, 0.15903, 0.05368, 0.08183], -3.69978);

const BANDS_LONG: [(f32, f32); 1] = [(40.0, 180.0)];
const BANDS_SHORT: [(f32, f32); 3] = [(200.0, 800.0), (1000.0, 5000.0), (6000.0, 16000.0)];

/// One window size: band levels in dB and band flatness for a frame centred on a sample.
struct Spectrum {
    n: usize,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<Complex32>,
    scratch: Vec<Complex32>,
    bins: Vec<(usize, usize)>,
}

impl Spectrum {
    fn new(n: usize, bands: &[(f32, f32)]) -> Self {
        let fft = FftPlanner::new().plan_fft_forward(n);
        // symmetric Hann window
        let window = (0..n).map(|i| (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (n - 1) as f64).cos()) as f32).collect();
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];
        let hz = SAMPLE_RATE as f32 / n as f32;
        // bins with lo <= f < hi
        let bins = bands.iter().map(|&(lo, hi)| ((lo / hz).ceil() as usize, ((hi / hz).ceil() as usize).min(n / 2 + 1))).collect();
        Self { n, fft, window, buf: vec![Complex32::default(); n], scratch, bins }
    }

    /// Level (dB) and flatness per band of the window centred on sample `c` of `pcm`.
    fn frame(&mut self, pcm: &[f32], c: usize, out: &mut [(f32, f32)]) {
        let half = self.n / 2;
        for (i, b) in self.buf.iter_mut().enumerate() {
            let j = (c + i).checked_sub(half);
            let s = j.and_then(|j| pcm.get(j)).copied().unwrap_or(0.0);
            *b = Complex32::new(s * self.window[i], 0.0);
        }
        self.fft.process_with_scratch(&mut self.buf, &mut self.scratch);
        for (o, &(a, b)) in out.iter_mut().zip(&self.bins) {
            let (mut sum, mut logs) = (0.0f64, 0.0f64);
            for k in a..b {
                let p = self.buf[k].norm_sqr() as f64 + 1e-12;
                sum += p;
                logs += p.ln();
            }
            let n = (b - a).max(1) as f64;
            let flat = (logs / n).exp() / (sum / n);
            *o = ((10.0 * sum.log10()) as f32, flat as f32);
        }
    }
}

/// Band levels of a whole track, one value per `STEP` samples, frame k centred on sample k·STEP.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bands {
    /// 40–180 Hz, dB
    pub low: Vec<f32>,
    /// 200–800 Hz, dB
    pub lowmid: Vec<f32>,
    /// 1–5 kHz, dB, and its flatness 0..1
    pub mid: Vec<f32>,
    pub mid_flat: Vec<f32>,
    /// 6–16 kHz, dB, and its flatness 0..1
    pub high: Vec<f32>,
    pub high_flat: Vec<f32>,
}

/// Frame-by-frame band analysis (the same for a whole track and for live input).
pub struct BandAnalyzer {
    long: Spectrum,
    short: Spectrum,
}

impl Default for BandAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl BandAnalyzer {
    pub fn new() -> Self {
        Self { long: Spectrum::new(LONG, &BANDS_LONG), short: Spectrum::new(SHORT, &BANDS_SHORT) }
    }

    /// Append the frame centred on sample `c` of `pcm` to `b`.
    pub fn push(&mut self, pcm: &[f32], c: usize, b: &mut Bands) {
        let mut l = [(0.0, 0.0); 1];
        let mut s = [(0.0, 0.0); 3];
        self.long.frame(pcm, c, &mut l);
        self.short.frame(pcm, c, &mut s);
        b.low.push(l[0].0);
        b.lowmid.push(s[0].0);
        b.mid.push(s[1].0);
        b.mid_flat.push(s[1].1);
        b.high.push(s[2].0);
        b.high_flat.push(s[2].1);
    }
}

pub fn track_bands(pcm: &[f32]) -> Bands {
    let mut a = BandAnalyzer::new();
    let mut b = Bands::default();
    for k in 0..pcm.len() / STEP {
        a.push(pcm, k * STEP, &mut b);
    }
    // levels far below the loudest are noise floor
    for v in [&mut b.low, &mut b.lowmid, &mut b.mid, &mut b.high] {
        let max = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        v.iter_mut().for_each(|x| *x = x.max(max - FLOOR_DB));
    }
    b
}

/// One drum hit: time in track seconds and strength 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub time: f64,
    pub strength: f32,
}

/// Hits of each drum, sorted by time.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrumHits(pub [Vec<Hit>; DRUMS]);

fn min_of(v: &[f32], a: isize, b: usize) -> f32 {
    v[a.max(0) as usize..b].iter().copied().fold(f32::INFINITY, f32::min)
}

fn max_of(v: &[f32], a: usize, b: usize) -> f32 {
    v[a..b].iter().copied().fold(f32::NEG_INFINITY, f32::max)
}

fn mean_of(v: &[f32], a: usize, b: usize) -> f32 {
    v[a..b].iter().sum::<f32>() / (b - a) as f32
}

/// Features of a candidate at frame k (see the model weights). `kick_ref`: level of the loud kicks.
fn features(d: usize, b: &Bands, k: usize, kick_ref: f32) -> Vec<f32> {
    let i = k as isize;
    match d {
        0 => {
            let peak = max_of(&b.low, k, k + 5);
            vec![peak - min_of(&b.low, i - 4, k), peak - kick_ref, b.low[k + 1] - b.low[k - 2], max_of(&b.lowmid, k, k + 3) - min_of(&b.lowmid, i - 3, k)]
        }
        1 => {
            let base = min_of(&b.mid, i - 4, k);
            let rm = max_of(&b.mid, k, k + 3) - base;
            let rh = max_of(&b.high, k, k + 3) - min_of(&b.high, i - 4, k);
            vec![rm, mean_of(&b.mid, k + 3, k + 8) - base, mean_of(&b.mid_flat, k + 1, k + 6) * 10.0, rm - rh, mean_of(&b.mid, k + 8, k + 15) - base, max_of(&b.lowmid, k, k + 4) - min_of(&b.lowmid, i - 4, k)]
        }
        _ => {
            let base = min_of(&b.high, i - 3, k);
            let rh = max_of(&b.high, k, k + 3) - base;
            let rm = max_of(&b.mid, k, k + 3) - min_of(&b.mid, i - 3, k);
            vec![rh, b.high[k + 3] - base, mean_of(&b.high_flat, k, k + 3) * 10.0, rh - rm, mean_of(&b.high, k + 6, k + 10) - base, max_of(&b.lowmid, k, k + 3) - min_of(&b.lowmid, i - 3, k)]
        }
    }
}

fn score(d: usize, f: &[f32]) -> f32 {
    let (w, b): (&[f32], f32) = match d {
        0 => (&KICK_MODEL.0, KICK_MODEL.1),
        1 => (&SNARE_MODEL.0, SNARE_MODEL.1),
        _ => (&HAT_MODEL.0, HAT_MODEL.1),
    };
    w.iter().zip(f).map(|(w, x)| w * x).sum::<f32>() + b
}

/// Score threshold for a sensitivity 0.5 (only clear hits) ..= 2 (also doubtful ones).
fn threshold(sensitivity: f32) -> f32 {
    -1.5 * sensitivity.clamp(0.25, 4.0).ln()
}

/// The level a drum's band follows: rises of it are the candidates.
fn series(d: usize, b: &Bands) -> &[f32] {
    match d {
        0 => &b.low,
        1 => &b.mid,
        _ => &b.high,
    }
}

/// Candidate frames: local peaks of the rise above a threshold that follows the music; of two
/// peaks closer than the drum's gap the larger one stays.
fn candidates(d: usize, v: &[f32]) -> Vec<usize> {
    let n = v.len();
    let rise: Vec<f32> = (0..n).map(|k| if k == 0 { 0.0 } else { (v[k] - v[k - 1]).max(0.0) }).collect();
    let mut pre = vec![0.0f64; n + 1];
    for (k, r) in rise.iter().enumerate() {
        pre[k + 1] = pre[k] + *r as f64;
    }
    let gap = (MIN_GAP[d] * FRAME_RATE) as usize;
    let mut out: Vec<usize> = Vec::new();
    for k in 8..n.saturating_sub(AFTER[d].max(8)) {
        let o = rise[k];
        // mean over a centred window, counted over its full width as at the edges of a track
        let mean = ((pre[(k + MEAN_HALF + 1).min(n)] - pre[k.saturating_sub(MEAN_HALF)]) / (2 * MEAN_HALF + 1) as f64) as f32;
        if o <= (mean * THRESHOLD).max(MIN_RISE) || o < rise[k - 1] || o <= rise[k + 1] || o < rise[k - 2] || o <= rise[k + 2] {
            continue;
        }
        match out.last_mut() {
            Some(last) if k - *last < gap => {
                if o > rise[*last] {
                    *last = k;
                }
            }
            _ => out.push(k),
        }
    }
    out
}

fn percentile(mut v: Vec<f32>, p: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    // linear interpolation between the closest ranks
    let x = p * (v.len() - 1) as f32;
    let (i, f) = (x.floor() as usize, x.fract());
    v[i] + (v[(i + 1).min(v.len() - 1)] - v[i]) * f
}

/// What a hit's strength follows: kicks their level against the loud kicks (feature 1), snares
/// and hi-hats the mean level of their attack. The rise would make a hi-hat in a clap's tail
/// weaker than one in silence, and the peak of a few noise bursts varies too much.
fn level(d: usize, b: &Bands, k: usize, f: &[f32]) -> f32 {
    if d == 0 { f[1] } else { mean_of(series(d, b), k, k + 5) }
}

/// Strength 0..1 from a hit's main feature against the strong hits of the track.
fn strength(main: f32, top: f32) -> f32 {
    (1.0 + (main - top) / 12.0).clamp(0.15, 1.0)
}

/// Pick the hits from a track's band levels.
pub fn pick(b: &Bands, sensitivity: f32) -> DrumHits {
    let mut hits = DrumHits::default();
    let th = threshold(sensitivity);
    for d in 0..DRUMS {
        let cands = candidates(d, series(d, b));
        let kick_ref = if d == 0 { percentile(cands.iter().map(|&k| max_of(&b.low, k, k + 5)).collect(), 0.95) } else { 0.0 };
        let found: Vec<(usize, Vec<f32>)> = cands
            .into_iter()
            .map(|k| (k, features(d, b, k, kick_ref)))
            .filter(|(k, f)| score(d, f) > th && (d != 2 || !masked(&hits.0, *k as f64 / FRAME_RATE, f)))
            .collect();
        let main = |k: usize, f: &[f32]| level(d, b, k, f);
        let top = if d == 0 { 0.0 } else { percentile(found.iter().map(|(k, f)| main(*k, f)).collect(), 0.9) };
        hits.0[d] = found.iter().map(|(k, f)| Hit { time: *k as f64 / FRAME_RATE, strength: strength(main(*k, f), top) }).collect();
    }
    hits
}

/// A clap, a snare and the click of a kick also rise above 6 kHz: a hi-hat candidate at time `t`
/// on top of one only counts when the high band rose clearly more than the mid band (feature 3).
fn masked(hits: &[Vec<Hit>; DRUMS], t: f64, f: &[f32]) -> bool {
    f[3] < HAT_OVER_OTHERS
        && hits[..2].iter().any(|v| {
            let i = v.partition_point(|h| h.time < t - MASK);
            v.get(i).is_some_and(|h| h.time <= t + MASK)
        })
}

const MASK: f64 = 0.015;
const HAT_OVER_OTHERS: f32 = 6.0;
/// Live, hi-hats are decided this many frames later than kicks and snares, so that a kick or
/// snare up to `MASK` after them is known.
const HAT_DELAY: usize = 3;

/// The same picking, live: samples come in as they are recorded and hits are reported once
/// the frames after them are in (about 100 ms late; the hit's age says so).
pub struct LiveDrums {
    analyzer: BandAnalyzer,
    pcm: Vec<f32>,
    /// sample index of `pcm[0]` in the stream
    pcm_start: usize,
    next_frame: usize,
    bands: Bands,
    /// frame index of `bands` entry 0
    band_start: usize,
    /// loudest level per band so far (slowly forgotten), level of the loud kicks, strong rises
    max: [f32; 4],
    kick_ref: f32,
    top: [f32; DRUMS],
    last: [Option<usize>; DRUMS],
    checked: usize,
}

impl Default for LiveDrums {
    fn default() -> Self {
        Self {
            analyzer: BandAnalyzer::new(),
            pcm: Vec::new(),
            pcm_start: 0,
            next_frame: 0,
            bands: Bands::default(),
            band_start: 0,
            max: [f32::NEG_INFINITY; 4],
            kick_ref: f32::NEG_INFINITY,
            top: [0.0; DRUMS],
            last: [None; DRUMS],
            checked: 0,
        }
    }
}

impl LiveDrums {
    /// Feed new samples (mono, `SAMPLE_RATE`). Returns the hits found: drum, age in seconds
    /// (how long ago it happened) and strength.
    pub fn feed(&mut self, samples: &[f32], sensitivity: f32) -> Vec<(usize, f32, f32)> {
        self.pcm.extend_from_slice(samples);
        let end = self.pcm_start + self.pcm.len();
        // a frame needs half the long window after its centre
        while self.next_frame * STEP + LONG / 2 <= end {
            let c = self.next_frame * STEP - self.pcm_start;
            self.analyzer.push(&self.pcm, c, &mut self.bands);
            let i = self.bands.low.len() - 1;
            for (m, v) in self.max.iter_mut().zip([&mut self.bands.low, &mut self.bands.lowmid, &mut self.bands.mid, &mut self.bands.high]) {
                *m = (*m - 0.002).max(v[i]);
                v[i] = v[i].max(*m - FLOOR_DB);
            }
            self.next_frame += 1;
        }
        let mut out = Vec::new();
        let th = threshold(sensitivity);
        let n = self.bands.low.len();
        // frames that now have enough frames after them
        while self.checked + AFTER[1] + 1 < n {
            let k = self.checked;
            self.checked += 1;
            if k < MEAN_HALF * 2 + 1 + HAT_DELAY {
                continue;
            }
            for (d, gap) in MIN_GAP.iter().enumerate() {
                let k = if d == 2 { k - HAT_DELAY } else { k };
                let v = series(d, &self.bands);
                let rise = |j: usize| (v[j] - v[j - 1]).max(0.0);
                // the threshold only looks back
                let mean = (k - MEAN_HALF * 2..k).map(rise).sum::<f32>() / (MEAN_HALF * 2) as f32;
                let o = rise(k);
                if o <= (mean * THRESHOLD).max(MIN_RISE) || o < rise(k - 1) || o <= rise(k + 1) || o < rise(k - 2) || o <= rise(k + 2) {
                    continue;
                }
                let frame = self.band_start + k;
                if self.last[d].is_some_and(|l| ((frame - l) as f64) < gap * FRAME_RATE) {
                    continue;
                }
                if d == 0 {
                    self.kick_ref = (self.kick_ref - 0.002).max(max_of(&self.bands.low, k, k + 5));
                }
                let f = features(d, &self.bands, k, self.kick_ref);
                if score(d, &f) <= th {
                    continue;
                }
                if d == 2 && f[3] < HAT_OVER_OTHERS && self.last[..2].iter().flatten().any(|&l| (frame as f64 - l as f64).abs() / FRAME_RATE <= MASK) {
                    continue;
                }
                let main = level(d, &self.bands, k, &f);
                if d > 0 {
                    self.top[d] = (self.top[d] - 0.05).max(main);
                }
                let s = strength(main, if d == 0 { 0.0 } else { self.top[d] });
                // age against the newest sample, not the newest frame: that lags half a long window
                let age = (end - frame * STEP) as f32 / SAMPLE_RATE as f32;
                self.last[d] = Some(frame);
                out.push((d, age, s));
            }
        }
        // keep a few seconds of frames and the samples the next frames need
        const KEEP: usize = 400;
        if self.bands.low.len() > KEEP * 2 {
            let cut = self.bands.low.len() - KEEP;
            for v in [&mut self.bands.low, &mut self.bands.lowmid, &mut self.bands.mid, &mut self.bands.mid_flat, &mut self.bands.high, &mut self.bands.high_flat] {
                v.drain(..cut);
            }
            self.band_start += cut;
            self.checked -= cut;
        }
        let need_from = (self.next_frame * STEP).saturating_sub(LONG);
        if need_from > self.pcm_start + LONG * 4 {
            let cut = need_from - self.pcm_start;
            self.pcm.drain(..cut);
            self.pcm_start += cut;
        }
        out
    }
}
