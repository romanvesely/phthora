//! The effect stack.

use crate::clock::{FrameTime, OutputMode};
use crate::modulator::{LiveInputs, Modulator};
use crate::params::{ParamKind, Value};
use crate::registry::{self, EffectDesc};
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
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
    /// uid of the group this effect belongs to
    pub group: Option<u64>,
}

/// How a group's result is combined with the image going into the group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupBlend {
    /// the group's result (laid over its input when isolated)
    #[default]
    Normal,
    /// the group's alpha cuts out its input
    Mask,
    /// the group's result goes behind its input
    Behind,
}

impl GroupBlend {
    pub const ALL: [GroupBlend; 3] = [GroupBlend::Normal, GroupBlend::Mask, GroupBlend::Behind];

    pub fn label(self) -> &'static str {
        match self {
            GroupBlend::Normal => "Normal",
            GroupBlend::Mask => "Mask",
            GroupBlend::Behind => "Behind",
        }
    }
}

/// A run of effects rendered together. Members are contiguous in the stack, one level deep.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub uid: u64,
    pub name: String,
    pub enabled: bool,
    pub locked: bool,
    pub expanded: bool,
    /// members start from a transparent image instead of the group's input
    pub isolate: bool,
    pub blend: GroupBlend,
}

impl Group {
    pub fn new() -> Self {
        Self { uid: new_uid(), name: "Group".into(), enabled: true, locked: false, expanded: true, isolate: false, blend: GroupBlend::Normal }
    }
}

impl Default for Group {
    fn default() -> Self {
        Self::new()
    }
}

/// The group settings a member renders with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroupRef {
    pub uid: u64,
    pub isolate: bool,
    pub blend: GroupBlend,
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
            group: None,
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
            group: None,
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
    pub group: Option<GroupRef>,
}

/// Same effect with the same values for this frame: rendering it again gives the same image.
impl PartialEq for ResolvedEffect {
    fn eq(&self, o: &Self) -> bool {
        self.uid == o.uid
            && self.desc.id == o.desc.id
            && self.values == o.values
            && self.time == o.time
            && self.seed == o.seed
            && self.data == o.data
            && self.triggers == o.triggers
            && self.group == o.group
    }
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
    pub groups: Vec<Group>,
}

/// Which texture a mask effect uses as the "un-effected" image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskSource {
    /// input of the enabled effect at this index in the resolved list
    InputOf(usize),
    /// input of the group whose first member is at this index (before isolation)
    GroupInput(usize),
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
        self.normalize();
    }

    /// Move an effect; it keeps its group as long as it stays next to it.
    pub fn move_to(&mut self, from: usize, to: usize) {
        if from < self.effects.len() && to < self.effects.len() && from != to {
            let e = self.effects.remove(from);
            self.effects.insert(to, e);
            if let Some(g) = self.effects[to].group {
                let near = |i: Option<usize>| i.and_then(|i| self.effects.get(i)).is_some_and(|n| n.group == Some(g));
                if !near(to.checked_sub(1)) && !near(Some(to + 1)) {
                    self.effects[to].group = None;
                }
            }
            self.normalize();
        }
    }

    /// Move an effect to `to` and make it a member of `group` (None: ungrouped).
    pub fn move_into(&mut self, from: usize, to: usize, group: Option<u64>) {
        if from >= self.effects.len() || to >= self.effects.len() {
            return;
        }
        let mut e = self.effects.remove(from);
        e.group = group.filter(|g| self.group(*g).is_some());
        self.effects.insert(to, e);
        self.normalize();
    }

    pub fn group(&self, uid: u64) -> Option<&Group> {
        self.groups.iter().find(|g| g.uid == uid)
    }

    pub fn group_mut(&mut self, uid: u64) -> Option<&mut Group> {
        self.groups.iter_mut().find(|g| g.uid == uid)
    }

    /// Locked itself or through its group.
    pub fn is_locked(&self, e: &EffectInstance) -> bool {
        e.locked || e.group.and_then(|g| self.group(g)).is_some_and(|g| g.locked)
    }

    /// Index range of a group's members.
    pub fn group_range(&self, uid: u64) -> Option<Range<usize>> {
        let start = self.effects.iter().position(|e| e.group == Some(uid))?;
        let len = self.effects[start..].iter().take_while(|e| e.group == Some(uid)).count();
        Some(start..start + len)
    }

    /// Put an effect into a new group of its own; returns the group's uid.
    pub fn make_group(&mut self, uid: u64) -> Option<u64> {
        let i = self.index_of(uid)?;
        let g = Group::new();
        let gid = g.uid;
        self.groups.push(g);
        self.effects[i].group = Some(gid);
        self.normalize();
        Some(gid)
    }

    /// Dissolve a group, keeping its effects in place.
    pub fn ungroup(&mut self, uid: u64) {
        for e in &mut self.effects {
            if e.group == Some(uid) {
                e.group = None;
            }
        }
        self.groups.retain(|g| g.uid != uid);
    }

    /// Delete a group and its effects.
    pub fn remove_group(&mut self, uid: u64) {
        self.effects.retain(|e| e.group != Some(uid));
        self.groups.retain(|g| g.uid != uid);
    }

    /// Copy a group with its effects right after it; returns the new group's uid.
    pub fn duplicate_group(&mut self, uid: u64) -> Option<u64> {
        let range = self.group_range(uid)?;
        let mut g = self.group(uid)?.clone();
        g.uid = new_uid();
        let gid = g.uid;
        let copies: Vec<EffectInstance> = self.effects[range.clone()]
            .iter()
            .map(|e| {
                let mut d = e.duplicate();
                d.group = Some(gid);
                d
            })
            .collect();
        self.groups.push(g);
        self.effects.splice(range.end..range.end, copies);
        Some(gid)
    }

    /// Stack blocks: a whole group or a single ungrouped effect.
    pub fn units(&self) -> Vec<Range<usize>> {
        let mut v = Vec::new();
        let mut i = 0;
        while i < self.effects.len() {
            let len = match self.effects[i].group {
                Some(g) => self.effects[i..].iter().take_while(|e| e.group == Some(g)).count(),
                None => 1,
            };
            v.push(i..i + len);
            i += len;
        }
        v
    }

    /// Swap a group with the block above (`up`) or below it.
    pub fn move_group(&mut self, uid: u64, up: bool) {
        let units = self.units();
        let Some(k) = units.iter().position(|r| self.effects[r.start].group == Some(uid)) else { return };
        let (a, b) = match up {
            true if k > 0 => (k - 1, k),
            false if k + 1 < units.len() => (k, k + 1),
            _ => return,
        };
        let (ra, rb) = (units[a].clone(), units[b].clone());
        // rotate the two adjacent blocks: b then a
        self.effects[ra.start..rb.end].rotate_left(ra.len());
    }

    /// Restore the invariants: members of a group are contiguous (gathered at the first
    /// member), unknown group ids are cleared and empty groups dropped.
    pub fn normalize(&mut self) {
        let known: Vec<u64> = self.groups.iter().map(|g| g.uid).collect();
        for e in &mut self.effects {
            if e.group.is_some_and(|g| !known.contains(&g)) {
                e.group = None;
            }
        }
        let mut done: Vec<u64> = Vec::new();
        let mut out = Vec::with_capacity(self.effects.len());
        let mut rest: Vec<Option<EffectInstance>> = std::mem::take(&mut self.effects).into_iter().map(Some).collect();
        for i in 0..rest.len() {
            let Some(e) = rest[i].take() else { continue };
            match e.group {
                Some(g) if !done.contains(&g) => {
                    done.push(g);
                    out.push(e);
                    for slot in rest.iter_mut().skip(i + 1) {
                        if slot.as_ref().is_some_and(|x| x.group == Some(g)) {
                            out.push(slot.take().unwrap());
                        }
                    }
                }
                _ => out.push(e),
            }
        }
        self.effects = out;
        self.groups.retain(|g| done.contains(&g.uid));
    }

    pub fn duplicate(&mut self, uid: u64) -> Option<u64> {
        let i = self.index_of(uid)?;
        let d = self.effects[i].duplicate();
        let nuid = d.uid;
        self.effects.insert(i + 1, d);
        Some(nuid)
    }

    /// Remove all unlocked effects (effects in a locked group count as locked).
    pub fn clear_unlocked(&mut self) {
        let keep: Vec<bool> = self.effects.iter().map(|e| self.is_locked(e)).collect();
        let mut k = keep.into_iter();
        self.effects.retain(|_| k.next().unwrap());
        self.normalize();
    }

    /// Disable all unlocked effects.
    pub fn disable_all(&mut self) {
        let locked: Vec<bool> = self.effects.iter().map(|e| self.is_locked(e)).collect();
        for (e, l) in self.effects.iter_mut().zip(locked) {
            if !l {
                e.enabled = false;
            }
        }
    }

    /// Random order of the blocks; groups move as a whole.
    pub fn shuffle(&mut self) {
        let mut units = self.units();
        units.shuffle(&mut rand::rng());
        let mut old: Vec<Option<EffectInstance>> = std::mem::take(&mut self.effects).into_iter().map(Some).collect();
        self.effects = units.into_iter().flat_map(|r| r.map(|i| old[i].take().unwrap()).collect::<Vec<_>>()).collect();
    }

    /// Replace the stack (keeps locked effects that are not in the new set).
    pub fn replace(&mut self, mut new: Chain, keep_locked: bool) {
        if keep_locked {
            let mut keep: Vec<EffectInstance> = self
                .effects
                .iter()
                .filter(|l| self.is_locked(l) && !new.effects.iter().any(|e| e.desc.id == l.desc.id && e.locked))
                .cloned()
                .collect();
            for g in &self.groups {
                if keep.iter().any(|e| e.group == Some(g.uid)) {
                    new.groups.push(g.clone());
                }
            }
            keep.append(&mut new.effects);
            new.effects = keep;
        }
        *self = new;
        self.normalize();
    }

    /// Chaos: drop unlocked effects, enable 3 random effects with random params.
    pub fn chaos(&mut self) {
        let mut rng = rand::rng();
        self.clear_unlocked();
        let mut pool: Vec<&'static EffectDesc> = registry::registry()
            .iter()
            .filter(|d| d.randomizable && d.available() && !self.effects.iter().any(|e| e.desc.id == d.id))
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

    /// Enabled effects resolved for this frame (members of disabled groups are skipped).
    pub fn resolve(&self, t: &FrameTime, inputs: &LiveInputs, midi: &HashMap<(String, u8), f64>) -> Vec<ResolvedEffect> {
        self.effects
            .iter()
            .filter(|e| e.enabled)
            .filter_map(|e| {
                let group = match e.group.and_then(|g| self.group(g)) {
                    Some(g) if !g.enabled => return None,
                    Some(g) => Some(GroupRef { uid: g.uid, isolate: g.isolate, blend: g.blend }),
                    None => None,
                };
                let mut r = e.resolve(t, inputs, midi);
                r.group = group;
                Some(r)
            })
            .collect()
    }
}

/// Mask routing for the resolved effect at `index`: the input of the effect directly
/// above. A mask right below a group cuts the whole group (the group's input); the first
/// effect inside a group has nothing above it.
pub fn mask_source(effects: &[ResolvedEffect], index: usize) -> MaskSource {
    let gid = |i: usize| effects[i].group.map(|g| g.uid);
    if index == 0 || index >= effects.len() {
        return MaskSource::Blank;
    }
    let (here, above) = (gid(index), gid(index - 1));
    if here == above {
        return MaskSource::InputOf(index - 1);
    }
    if here.is_some() {
        return MaskSource::Blank;
    }
    let mut start = index - 1;
    while start > 0 && gid(start - 1) == above {
        start -= 1;
    }
    MaskSource::GroupInput(start)
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
    fn chaos_keeps_locked() {
        let mut c = Chain::default();
        c.add("pixelate");
        c.effects[0].locked = true;
        c.add("bulge");
        c.chaos();
        assert_eq!(c.effects.len(), 4);
        assert_eq!(c.effects[0].desc.id, "pixelate");
        assert!(c.effects.iter().all(|e| e.desc.available()));
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
    }

    fn ids(c: &Chain) -> Vec<(&str, Option<u64>)> {
        c.effects.iter().map(|e| (e.desc.id.as_str(), e.group)).collect()
    }

    #[test]
    fn groups_stay_contiguous() {
        let mut c = Chain::default();
        let a = c.add("bad_tv").unwrap();
        let b = c.add("pixelate").unwrap();
        c.add("bulge");
        let g = c.make_group(a).unwrap();
        // pixelate joins the group, bulge moves into it from below
        c.move_into(1, 1, Some(g));
        c.move_into(2, 1, Some(g));
        assert_eq!(ids(&c), vec![("bad_tv", Some(g)), ("bulge", Some(g)), ("pixelate", Some(g))]);
        // dragged away from the group: leaves it
        let mut d = c.clone();
        d.add("glow");
        d.move_to(0, 3);
        assert_eq!(ids(&d)[3], ("bad_tv", None));
        // moved within the group: stays
        c.move_to(0, 1);
        assert_eq!(ids(&c), vec![("bulge", Some(g)), ("bad_tv", Some(g)), ("pixelate", Some(g))]);
        // scattered members are gathered at the first one
        c.add("glow");
        c.effects.swap(2, 3);
        c.normalize();
        assert_eq!(ids(&c), vec![("bulge", Some(g)), ("bad_tv", Some(g)), ("pixelate", Some(g)), ("glow", None)]);
        assert_eq!(c.group_range(g), Some(0..3));
        c.move_group(g, false);
        assert_eq!(c.effects[0].desc.id, "glow");
        assert_eq!(c.group_range(g), Some(1..4));
        let g2 = c.duplicate_group(g).unwrap();
        assert_eq!(c.group_range(g2), Some(4..7));
        assert_ne!(c.effects[4].uid, c.effects[1].uid);
        c.shuffle();
        assert_eq!(c.group_range(g).map(|r| r.len()), Some(3));
        assert_eq!(c.group_range(g2).map(|r| r.len()), Some(3));
        c.ungroup(g2);
        assert_eq!(c.groups.len(), 1);
        c.remove_group(g);
        assert_eq!(c.effects.len(), 4);
        // the last member leaving drops the group
        let gb = c.make_group(c.effects[0].uid).unwrap();
        let first = c.effects[0].uid;
        c.remove(first);
        assert!(c.group(gb).is_none());
        let _ = b;
    }

    #[test]
    fn locked_groups_survive_chaos_and_disabled_groups_do_not_render() {
        let mut c = Chain::default();
        let a = c.add("pixelate").unwrap();
        c.add("bulge");
        let g = c.make_group(a).unwrap();
        c.group_mut(g).unwrap().locked = true;
        c.chaos();
        assert_eq!(c.effects[0].uid, a);
        assert_eq!(c.groups.len(), 1);
        let t = TimingSettings::default().at_seconds(0.0);
        let r = c.resolve(&t, &LiveInputs::default(), &HashMap::new());
        assert_eq!(r[0].group.map(|g| g.uid), Some(g));
        assert_eq!(r.len(), 4);
        c.group_mut(g).unwrap().enabled = false;
        assert_eq!(c.resolve(&t, &LiveInputs::default(), &HashMap::new()).len(), 3);
    }

    #[test]
    fn mask_routing_with_groups() {
        let mut c = Chain::default();
        for id in ["glow", "pixelate", "mask", "bulge", "mask", "mask"] {
            c.add(id);
        }
        // glow | [pixelate, mask, bulge] | mask | mask
        let g = c.make_group(c.effects[1].uid).unwrap();
        c.effects[2].group = Some(g);
        c.effects[3].group = Some(g);
        let t = TimingSettings::default().at_seconds(0.0);
        let r = c.resolve(&t, &LiveInputs::default(), &HashMap::new());
        assert_eq!(mask_source(&r, 0), MaskSource::Blank);
        assert_eq!(mask_source(&r, 1), MaskSource::Blank, "first in a group");
        assert_eq!(mask_source(&r, 2), MaskSource::InputOf(1));
        assert_eq!(mask_source(&r, 4), MaskSource::GroupInput(1), "right below the group");
        assert_eq!(mask_source(&r, 5), MaskSource::InputOf(4));
    }
}
