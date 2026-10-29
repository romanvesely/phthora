//! The effect stack.

use crate::clock::{FrameTime, OutputMode};
use crate::modulator::{LiveInputs, Modulator};
use crate::params::{ParamKind, Value};
use crate::registry::{self, EffectDesc};
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_UID: AtomicU64 = AtomicU64::new(1);

pub fn new_uid() -> u64 {
    NEXT_UID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Debug)]
pub struct EffectInstance {
    /// unique per session, used to keep GPU state
    pub uid: u64,
    pub desc: &'static EffectDesc,
    pub display_name: Option<String>,
    pub enabled: bool,
    pub locked: bool,
    pub expanded: bool,
    /// time offset 0..1 so effects don't all loop in phase
    pub seed: f64,
    pub values: BTreeMap<String, Value>,
    pub modulators: BTreeMap<String, Modulator>,
    /// opaque per-effect data (e.g. painted masks), saved in presets
    pub data: BTreeMap<String, String>,
    /// bumped when a "button" param is pressed: (key, counter)
    pub triggers: BTreeMap<String, u64>,
}

impl EffectInstance {
    pub fn new(desc: &'static EffectDesc) -> Self {
        let mut values = BTreeMap::new();
        for p in &desc.params {
            if let Some(v) = p.default_value() {
                values.insert(p.key.clone(), v);
            }
        }
        Self {
            uid: new_uid(),
            desc,
            display_name: None,
            enabled: true,
            locked: false,
            expanded: true,
            seed: rand::random(),
            values,
            modulators: BTreeMap::new(),
            data: BTreeMap::new(),
            triggers: BTreeMap::new(),
        }
    }

    pub fn by_id(id: &str) -> Option<Self> {
        registry::find(id).map(Self::new)
    }

    pub fn name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.desc.name)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn f(&self, key: &str) -> f32 {
        self.values.get(key).map(|v| v.as_f32()).unwrap_or(0.0)
    }

    pub fn set(&mut self, key: &str, v: Value) {
        self.values.insert(key.to_string(), v);
    }

    pub fn reset(&mut self, key: &str) {
        if let Some(v) = self.desc.param(key).and_then(|p| p.default_value()) {
            self.values.insert(key.to_string(), v);
        }
    }

    pub fn trigger(&mut self, key: &str) {
        *self.triggers.entry(key.to_string()).or_default() += 1;
    }

    /// Randomize all randomizable float/combo params within their chaos range.
    pub fn randomize(&mut self, rng: &mut impl Rng) {
        for p in &self.desc.params {
            if p.hidden {
                continue;
            }
            match &p.kind {
                ParamKind::Float { step, no_random: false, image_only: false, .. } => {
                    let (lo, hi) = p.rand_bounds().unwrap();
                    let mut v = if hi > lo { rng.random_range(lo..=hi) } else { lo };
                    if *step > 0.0 {
                        v = (v / step).round() * step;
                    }
                    self.values.insert(p.key.clone(), Value::Float(v));
                }
                ParamKind::Combo { values, no_random: false, .. } if !values.is_empty() => {
                    let v = values[rng.random_range(0..values.len())].clone();
                    self.values.insert(p.key.clone(), Value::Text(v));
                }
                _ => {}
            }
        }
    }

    /// A copy with a new identity and fresh random seeds.
    pub fn duplicate(&self) -> Self {
        let mut d = self.clone();
        d.uid = new_uid();
        d.seed = rand::random();
        for m in d.modulators.values_mut() {
            m.seed = rand::random::<f64>() * 9999.0;
        }
        d
    }

    /// Effective parameter values for this frame (modulators applied).
    pub fn resolve(&self, t: &FrameTime, inputs: &LiveInputs, midi: &HashMap<(String, u8), f64>) -> ResolvedEffect {
        let mut values = self.values.clone();
        if t.mode == OutputMode::Video {
            for (key, m) in &self.modulators {
                if !m.enabled {
                    continue;
                }
                let Some(p) = self.desc.param(key) else { continue };
                let ParamKind::Float { min, max, step, .. } = p.kind else { continue };
                let mv = m.midi_cc.and_then(|cc| midi.get(&(m.midi_device.clone(), cc)).copied());
                if let Some(v) = m.eval(t, inputs, mv, min, max, step) {
                    values.insert(key.clone(), Value::Float(v));
                }
            }
        }
        let time = match t.mode {
            OutputMode::Image => 0.0,
            OutputMode::Video => ((t.loop_progress + self.seed) % 1.0) as f32,
        };
        ResolvedEffect {
            uid: self.uid,
            desc: self.desc,
            values,
            time,
            seed: self.seed,
            data: self.data.clone(),
            triggers: self.triggers.clone(),
        }
    }
}

/// An effect instance with its per-frame values.
#[derive(Clone, Debug)]
pub struct ResolvedEffect {
    pub uid: u64,
    pub desc: &'static EffectDesc,
    pub values: BTreeMap<String, Value>,
    /// value for the `time` uniform (loop phase + seed, or 0 in image mode)
    pub time: f32,
    /// per-instance random seed (0..1), stable across frames and saved in presets
    pub seed: f64,
    pub data: BTreeMap<String, String>,
    pub triggers: BTreeMap<String, u64>,
}

impl ResolvedEffect {
    pub fn f(&self, key: &str) -> f32 {
        self.values.get(key).map(|v| v.as_f32()).unwrap_or(0.0)
    }
    pub fn b(&self, key: &str) -> bool {
        self.values.get(key).map(|v| v.as_bool()).unwrap_or(false)
    }
    pub fn s(&self, key: &str) -> &str {
        self.values.get(key).map(|v| v.as_str()).unwrap_or("")
    }
}

#[derive(Clone, Debug, Default)]
pub struct Chain {
    pub effects: Vec<EffectInstance>,
    undo: Vec<Vec<EffectInstance>>,
}

/// Which texture a mask effect uses as the "un-effected" image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskSource {
    /// input of the enabled effect at this index in the resolved list
    InputOf(usize),
    /// nothing above: transparent
    Blank,
}

impl Chain {
    pub fn add(&mut self, id: &str) -> Option<u64> {
        let e = EffectInstance::by_id(id)?;
        let uid = e.uid;
        self.effects.push(e);
        Some(uid)
    }

    pub fn index_of(&self, uid: u64) -> Option<usize> {
        self.effects.iter().position(|e| e.uid == uid)
    }

    pub fn get_mut(&mut self, uid: u64) -> Option<&mut EffectInstance> {
        self.effects.iter_mut().find(|e| e.uid == uid)
    }

    pub fn remove(&mut self, uid: u64) {
        self.effects.retain(|e| e.uid != uid);
    }

    pub fn move_to(&mut self, from: usize, to: usize) {
        if from < self.effects.len() && to < self.effects.len() && from != to {
            let e = self.effects.remove(from);
            self.effects.insert(to, e);
        }
    }

    pub fn duplicate(&mut self, uid: u64) -> Option<u64> {
        let i = self.index_of(uid)?;
        let d = self.effects[i].duplicate();
        let nuid = d.uid;
        self.effects.insert(i + 1, d);
        Some(nuid)
    }

    /// Remove all unlocked effects.
    pub fn clear_unlocked(&mut self) {
        self.effects.retain(|e| e.locked);
    }

    /// Disable all unlocked effects.
    pub fn disable_all(&mut self) {
        for e in &mut self.effects {
            if !e.locked {
                e.enabled = false;
            }
        }
    }

    pub fn shuffle(&mut self) {
        self.effects.shuffle(&mut rand::rng());
    }

    fn push_undo(&mut self) {
        self.undo.push(self.effects.clone());
        if self.undo.len() > 10 {
            self.undo.remove(0);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.effects = prev;
        }
    }

    /// Replace the stack (keeps locked effects that are not in the new set).
    pub fn replace(&mut self, mut effects: Vec<EffectInstance>, keep_locked: bool) {
        if keep_locked {
            let locked: Vec<_> = self.effects.iter().filter(|e| e.locked).cloned().collect();
            for l in locked.into_iter().rev() {
                if !effects.iter().any(|e| e.desc.id == l.desc.id && e.locked) {
                    effects.insert(0, l);
                }
            }
        }
        self.effects = effects;
    }

    /// Chaos: drop unlocked effects, enable 3 random effects with random params (undoable).
    pub fn chaos(&mut self) {
        self.push_undo();
        let mut rng = rand::rng();
        self.clear_unlocked();
        let mut pool: Vec<&'static EffectDesc> = registry::registry()
            .iter()
            .filter(|d| d.randomizable && !self.effects.iter().any(|e| e.desc.id == d.id))
            .collect();
        pool.shuffle(&mut rng);
        let mut picked: Vec<&'static EffectDesc> = pool.into_iter().take(3).collect();
        // keep catalog order
        let order = |d: &EffectDesc| registry::registry().iter().position(|x| x.id == d.id).unwrap_or(0);
        picked.sort_by_key(|d| order(d));
        for d in picked {
            let mut e = EffectInstance::new(d);
            e.randomize(&mut rng);
            e.expanded = false;
            self.effects.push(e);
        }
    }

    /// Enabled effects resolved for this frame.
    pub fn resolve(&self, t: &FrameTime, inputs: &LiveInputs, midi: &HashMap<(String, u8), f64>) -> Vec<ResolvedEffect> {
        self.effects.iter().filter(|e| e.enabled).map(|e| e.resolve(t, inputs, midi)).collect()
    }
}

/// Mask routing for the enabled effect at `index`: the input of the effect directly above.
pub fn mask_source(index: usize) -> MaskSource {
    if index == 0 { MaskSource::Blank } else { MaskSource::InputOf(index - 1) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::TimingSettings;

    #[test]
    fn stack_ops() {
        let mut c = Chain::default();
        let a = c.add("bad_tv").unwrap();
        let b = c.add("pixelate").unwrap();
        c.move_to(1, 0);
        assert_eq!(c.effects[0].uid, b);
        let d = c.duplicate(a).unwrap();
        assert_eq!(c.effects[2].uid, d);
        assert_eq!(c.effects[2].desc.id, "bad_tv");
        c.remove(b);
        assert_eq!(c.effects.len(), 2);
    }

    #[test]
    fn chaos_and_undo() {
        let mut c = Chain::default();
        c.add("pixelate");
        c.effects[0].locked = true;
        c.add("bulge");
        c.chaos();
        assert_eq!(c.effects.len(), 4);
        assert_eq!(c.effects[0].desc.id, "pixelate");
        c.undo();
        assert_eq!(c.effects.len(), 2);
        assert_eq!(c.effects[1].desc.id, "bulge");
    }

    #[test]
    fn resolve_time_and_modulators() {
        let mut c = Chain::default();
        let uid = c.add("bad_tv").unwrap();
        let e = c.get_mut(uid).unwrap();
        e.seed = 0.5;
        e.modulators.insert(
            "distortion".into(),
            Modulator { enabled: true, shape: crate::modulator::Shape::Const, max: 1.0, seed: 0.0, ..Default::default() },
        );
        let ts = TimingSettings::default();
        let t = ts.at_seconds(1.0);
        let r = c.resolve(&t, &LiveInputs::default(), &HashMap::new());
        assert!((r[0].time - ((t.loop_progress + 0.5) % 1.0) as f32).abs() < 1e-6);
        assert_eq!(r[0].f("distortion"), 1.0);
        assert_eq!(mask_source(0), MaskSource::Blank);
        assert_eq!(mask_source(2), MaskSource::InputOf(1));
    }
}
