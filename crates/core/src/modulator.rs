//! Parameter modulators (LFO shapes, noise, keyframes, audio, MIDI).

use crate::clock::FrameTime;
use crate::noise::{lerp, loop_noise1d, noise1d, random1d};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    #[default]
    Linear,
    Sine,
    Bounce,
    Step,
    Noise,
    Rand,
    Const,
    Audio,
    Midi,
    Keys,
}

impl Shape {
    pub const ALL: [Shape; 10] = [
        Shape::Linear, Shape::Sine, Shape::Bounce, Shape::Step, Shape::Noise,
        Shape::Rand, Shape::Const, Shape::Audio, Shape::Midi, Shape::Keys,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Shape::Linear => "linear",
            Shape::Sine => "sine",
            Shape::Bounce => "bounce",
            Shape::Step => "step",
            Shape::Noise => "noise",
            Shape::Rand => "rand",
            Shape::Const => "const",
            Shape::Audio => "audio",
            Shape::Midi => "midi",
            Shape::Keys => "keys",
        }
    }
}

/// Audio band a modulator follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Band {
    #[default]
    All,
    Low,
    Mid,
    Hi,
    /// user band n (0-based) from the audio settings
    Custom(u8),
}

/// Built-in bands plus user bands.
pub const MAX_BANDS: usize = 8;

impl Band {
    pub const BUILTIN: [Band; 4] = [Band::All, Band::Low, Band::Mid, Band::Hi];

    pub fn index(self) -> usize {
        match self {
            Band::All => 0,
            Band::Low => 1,
            Band::Mid => 2,
            Band::Hi => 3,
            Band::Custom(n) => (4 + n as usize).min(MAX_BANDS - 1),
        }
    }

    pub fn name(self) -> String {
        match self {
            Band::All => "all".into(),
            Band::Low => "low".into(),
            Band::Mid => "mid".into(),
            Band::Hi => "hi".into(),
            Band::Custom(n) => format!("custom{}", n as u32 + 1),
        }
    }

    pub fn parse(s: &str) -> Option<Band> {
        Some(match s {
            "all" => Band::All,
            "low" => Band::Low,
            "mid" => Band::Mid,
            "hi" => Band::Hi,
            _ => Band::Custom(s.strip_prefix("custom")?.parse::<u8>().ok()?.checked_sub(1)?),
        })
    }
}

impl Serialize for Band {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.name())
    }
}

impl<'de> Deserialize<'de> for Band {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(Band::parse(&s).unwrap_or_default())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Modulator {
    pub enabled: bool,
    pub shape: Shape,
    pub speed: f64,
    /// modulator output range as fraction of the parameter range
    pub min: f64,
    pub max: f64,
    /// true: speed relative to effect loop (4 cycles per loop); false: relative to output duration
    #[serde(rename = "loop")]
    pub looped: bool,
    pub chance: f64,
    pub start: f64,
    pub end: f64,
    /// missing in a hand-written preset: 0, so loading stays deterministic
    #[serde(default)]
    pub seed: f64,
    pub band: Band,
    pub midi_cc: Option<u8>,
    pub midi_device: String,
    /// (time 0..1, value 0..1), sorted by time
    pub keyframes: Vec<[f64; 2]>,
}

impl Default for Modulator {
    fn default() -> Self {
        Self {
            enabled: false,
            shape: Shape::Linear,
            speed: 1.0,
            min: 0.0,
            max: 1.0,
            looped: true,
            chance: 1.0,
            start: 0.0,
            end: 1.0,
            seed: rand::random::<f64>() * 9999.0,
            band: Band::All,
            midi_cc: None,
            midi_device: String::new(),
            keyframes: Vec::new(),
        }
    }
}

/// External live inputs a modulator may read.
#[derive(Clone, Copy, Debug, Default)]
pub struct LiveInputs {
    /// audio band levels 0..1 (all, low, mid, hi, user bands)
    pub audio: [f32; MAX_BANDS],
    /// true when there is no meaningful duration (webcam/mic live): start/end disabled
    pub live: bool,
}

impl Modulator {
    /// Normalized 0..1 value before mapping to the parameter range.
    /// `midi_value` is the last received MIDI value (0..1) for this modulator.
    pub fn eval_normalized(&self, t: &FrameTime, inputs: &LiveInputs, midi_value: Option<f64>) -> Option<f64> {
        if self.shape == Shape::Midi {
            return midi_value;
        }
        if self.shape == Shape::Keys {
            return Some(keyframe_value(&self.keyframes, t.export_progress));
        }
        let time = if self.looped { t.loop_progress * 4.0 * self.speed } else { t.export_progress * self.speed };
        let tm = time % 1.0;
        let mut val = match self.shape {
            Shape::Linear => tm,
            Shape::Sine => ((time * std::f64::consts::TAU).sin() + 1.0) / 2.0,
            Shape::Bounce => {
                if tm < 0.0 { 0.0 } else { 8.0 * tm * (1.0 - 8.0 * tm).exp() }
            }
            Shape::Step => tm.round(),
            Shape::Rand => random1d((t.export_duration * t.export_progress * self.speed * 4.0).floor() + self.seed),
            Shape::Noise => loop_noise1d(t.export_progress, self.speed * t.export_duration / 16.0, self.seed) / 2.0 + 0.5,
            Shape::Audio => inputs.audio[self.band.index()] as f64,
            Shape::Const => self.max,
            Shape::Midi | Shape::Keys => unreachable!(),
        };
        if !inputs.live {
            let p = t.export_progress.clamp(0.0, 1.0);
            if p < self.start || p > self.end {
                val = 0.0;
            }
        }
        if self.shape != Shape::Const {
            val = lerp(val, self.min, self.max);
        }
        if noise1d(time * 2.0 + self.seed + 7987.656) > self.chance {
            val = 0.0;
        }
        Some(val)
    }

    /// Final parameter value in [pmin, pmax] (with integer rounding for step 1).
    pub fn eval(&self, t: &FrameTime, inputs: &LiveInputs, midi_value: Option<f64>, pmin: f32, pmax: f32, step: f32) -> Option<f32> {
        let n = self.eval_normalized(t, inputs, midi_value)?;
        let mut v = lerp(n, pmin as f64, pmax as f64) as f32;
        if step == 1.0 {
            v = v.round();
        }
        Some(v)
    }
}

/// Piecewise linear; constant outside the first/last point; 1 without points.
pub fn keyframe_value(points: &[[f64; 2]], t: f64) -> f64 {
    match points {
        [] => 1.0,
        [p] => p[1],
        _ => {
            if t <= points[0][0] {
                return points[0][1];
            }
            for w in points.windows(2) {
                let (a, b) = (w[0], w[1]);
                if t <= b[0] {
                    let span = b[0] - a[0];
                    if span <= 0.0 {
                        return b[1];
                    }
                    return a[1] + (b[1] - a[1]) * (t - a[0]) / span;
                }
            }
            points[points.len() - 1][1]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::TimingSettings;

    fn m(shape: Shape) -> Modulator {
        Modulator { enabled: true, shape, seed: 1.0, ..Default::default() }
    }

    #[test]
    fn shapes() {
        let ts = TimingSettings { duration: 4.0, ..Default::default() };
        let t = ts.at_seconds(0.5); // loop progress 0.125 -> time 0.5
        let li = LiveInputs::default();
        assert!((m(Shape::Linear).eval_normalized(&t, &li, None).unwrap() - 0.5).abs() < 1e-9);
        assert!((m(Shape::Sine).eval_normalized(&t, &li, None).unwrap() - 0.5).abs() < 1e-9);
        assert_eq!(m(Shape::Step).eval_normalized(&t, &li, None).unwrap(), 1.0);
        let c = Modulator { max: 0.7, ..m(Shape::Const) };
        assert!((c.eval_normalized(&t, &li, None).unwrap() - 0.7).abs() < 1e-9);
        // mapping to param range + rounding
        let v = m(Shape::Linear).eval(&t, &li, None, 0.0, 10.0, 1.0).unwrap();
        assert_eq!(v, 5.0);
    }

    #[test]
    fn bands() {
        for b in [Band::All, Band::Hi, Band::Custom(0), Band::Custom(3)] {
            assert_eq!(Band::parse(&b.name()), Some(b));
            let j = serde_json::to_string(&b).unwrap();
            assert_eq!(serde_json::from_str::<Band>(&j).unwrap(), b);
        }
        assert_eq!(serde_json::from_str::<Band>("\"nonsense\"").unwrap(), Band::All);
        let mut li = LiveInputs::default();
        li.audio[5] = 0.75;
        let m = Modulator { shape: Shape::Audio, band: Band::Custom(1), ..m(Shape::Audio) };
        let ts = TimingSettings::default();
        assert!((m.eval_normalized(&ts.at_seconds(0.1), &li, None).unwrap() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn start_end_and_keys() {
        let ts = TimingSettings { duration: 4.0, ..Default::default() };
        let li = LiveInputs::default();
        let mm = Modulator { start: 0.5, ..m(Shape::Const) };
        assert_eq!(mm.eval_normalized(&ts.at_seconds(1.0), &li, None).unwrap(), 0.0);
        let k = vec![[0.0, 0.0], [0.5, 1.0], [1.0, 0.0]];
        assert!((keyframe_value(&k, 0.25) - 0.5).abs() < 1e-9);
        assert_eq!(keyframe_value(&[], 0.3), 1.0);
    }
}
