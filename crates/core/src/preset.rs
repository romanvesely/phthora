//! Preset files (`.phthora.json`).

use crate::chain::{Chain, EffectInstance, Group, GroupBlend};
use crate::modulator::Modulator;
use crate::params::Value;
use crate::registry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const EXTENSION: &str = "phthora.json";
/// Version 2 adds groups; version 1 files load unchanged.
pub const VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PresetFile {
    pub version: u32,
    pub effects: Vec<PresetEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<PresetGroup>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PresetGroup {
    /// referenced by the members' `group`, only within the file
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "yes")]
    pub expanded: bool,
    #[serde(default)]
    pub isolate: bool,
    #[serde(default)]
    pub blend: GroupBlend,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PresetEffect {
    pub effect: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub expanded: bool,
    #[serde(default)]
    pub seed: f64,
    #[serde(default)]
    pub params: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub modulators: BTreeMap<String, Modulator>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<u32>,
}

fn yes() -> bool {
    true
}

pub fn to_preset(chain: &Chain) -> PresetFile {
    let gid = |uid: u64| chain.groups.iter().position(|g| g.uid == uid).map(|i| i as u32 + 1);
    PresetFile {
        version: VERSION,
        groups: chain
            .groups
            .iter()
            .enumerate()
            .map(|(i, g)| PresetGroup {
                id: i as u32 + 1,
                name: g.name.clone(),
                enabled: g.enabled,
                locked: g.locked,
                expanded: g.expanded,
                isolate: g.isolate,
                blend: g.blend,
            })
            .collect(),
        effects: chain
            .effects
            .iter()
            .map(|e| PresetEffect {
                effect: e.desc.id.clone(),
                name: e.display_name.clone(),
                enabled: e.enabled,
                locked: e.locked,
                expanded: e.expanded,
                seed: e.seed,
                params: e.values.iter().map(|(k, v)| (k.clone(), v.to_json())).collect(),
                // disabled modulators keep their settings, like in the editor
                modulators: e.modulators.clone(),
                data: e.data.clone(),
                group: e.group.and_then(gid),
            })
            .collect(),
    }
}

/// Unknown effects / params are skipped; returns the stack and a list of warnings.
pub fn from_preset(p: &PresetFile) -> (Chain, Vec<String>) {
    let mut out = Vec::new();
    let mut warn = Vec::new();
    let groups: Vec<(u32, Group)> = p
        .groups
        .iter()
        .map(|pg| {
            let mut g = Group::new();
            if !pg.name.is_empty() {
                g.name = pg.name.clone();
            }
            g.enabled = pg.enabled;
            g.locked = pg.locked;
            g.expanded = pg.expanded;
            g.isolate = pg.isolate;
            g.blend = pg.blend;
            (pg.id, g)
        })
        .collect();
    for pe in &p.effects {
        let Some(desc) = registry::find(&pe.effect).filter(|d| d.available()) else {
            warn.push(format!("unknown effect '{}'", pe.effect));
            continue;
        };
        let mut e = EffectInstance::new(desc);
        e.display_name = pe.name.clone();
        e.enabled = pe.enabled;
        e.locked = pe.locked;
        e.expanded = pe.expanded;
        e.seed = pe.seed;
        for (k, v) in &pe.params {
            match desc.param(k).and_then(|d| Value::from_json(d, v)) {
                Some(val) => {
                    e.values.insert(k.clone(), val);
                }
                None => warn.push(format!("{}: ignored param '{}'", desc.name, k)),
            }
        }
        e.modulators = pe.modulators.iter().filter(|(k, _)| desc.param(k).is_some_and(|p| p.is_float())).map(|(k, m)| (k.clone(), m.clone())).collect();
        e.data = pe.data.clone();
        e.group = pe.group.and_then(|id| groups.iter().find(|(i, _)| *i == id)).map(|(_, g)| g.uid);
        out.push(e);
    }
    let mut chain = Chain { effects: out, groups: groups.into_iter().map(|(_, g)| g).collect() };
    chain.normalize();
    (chain, warn)
}

pub fn to_json(chain: &Chain) -> String {
    serde_json::to_string_pretty(&to_preset(chain)).expect("serializable")
}

pub fn from_json(s: &str) -> Result<(Chain, Vec<String>), serde_json::Error> {
    let p: PresetFile = serde_json::from_str(s)?;
    Ok(from_preset(&p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Chain;

    #[test]
    fn roundtrip() {
        let mut c = Chain::default();
        let uid = c.add("color_gradient").unwrap();
        c.add("duotone");
        let e = c.get_mut(uid).unwrap();
        e.set("angle", Value::Float(0.25));
        e.modulators.insert("angle".into(), Modulator { enabled: true, ..Default::default() });
        let s = to_json(&c);
        let (back, warn) = from_json(&s).unwrap();
        assert!(warn.is_empty(), "{warn:?}");
        let back = back.effects;
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].values, c.effects[0].values);
        assert_eq!(back[0].seed, c.effects[0].seed);
        assert_eq!(back[0].modulators, c.effects[0].modulators);
    }

    #[test]
    fn missing_fields_are_deterministic() {
        let s = r#"{"version":1,"effects":[{"effect":"rgb_shift","modulators":{"amount":{"enabled":false,"shape":"noise"}}}]}"#;
        let (a, _) = from_json(s).unwrap();
        let (b, _) = from_json(s).unwrap();
        let (a, b) = (a.effects, b.effects);
        assert_eq!(a[0].seed, b[0].seed);
        assert_eq!(a[0].modulators["amount"].seed, b[0].modulators["amount"].seed);
        // disabled modulators survive a save
        let (c, _) = from_json(&to_json(&Chain { effects: a, groups: vec![] })).unwrap();
        assert_eq!(c.effects[0].modulators["amount"].shape, crate::modulator::Shape::Noise);
    }

    #[test]
    fn midi_mapping_roundtrip() {
        let mut c = crate::chain::Chain::default();
        let uid = c.add("rgb_shift").unwrap();
        let m = c.get_mut(uid).unwrap().modulators.entry("amount".into()).or_default();
        m.enabled = true;
        m.shape = crate::modulator::Shape::Midi;
        m.midi_cc = Some(21);
        m.midi_device = "nanoKONTROL2".into();
        let (d, _) = from_json(&to_json(&c)).unwrap();
        let d = d.effects;
        let m = &d[0].modulators["amount"];
        assert_eq!((m.shape, m.midi_cc, m.midi_device.as_str()), (crate::modulator::Shape::Midi, Some(21), "nanoKONTROL2"));
        // the mapped value sets the parameter across its whole range
        let t = crate::TimingSettings::default().at_frame(3);
        let midi = std::collections::HashMap::from([(("nanoKONTROL2".to_string(), 21u8), 1.0)]);
        let r = d[0].resolve(&t, &Default::default(), &midi);
        let max = match d[0].desc.param("amount").unwrap().kind {
            crate::params::ParamKind::Float { max, .. } => max,
            _ => unreachable!(),
        };
        assert_eq!(r.values["amount"], crate::params::Value::Float(max));
        // nothing received yet: the slider value stays
        let r = d[0].resolve(&t, &Default::default(), &Default::default());
        assert_eq!(r.values["amount"], d[0].values["amount"]);
    }

    #[test]
    fn unavailable_effects_are_skipped() {
        let s = r#"{"version":1,"effects":[{"effect":"definitely_not_an_effect"},{"effect":"pixelate"}]}"#;
        let (fx, warn) = from_json(s).unwrap();
        assert_eq!(fx.effects.len(), 1);
        assert_eq!(warn.len(), 1);
    }

    #[test]
    fn groups_roundtrip() {
        let mut c = Chain::default();
        let a = c.add("pixelate").unwrap();
        c.add("bulge");
        c.add("glow");
        let g = c.make_group(a).unwrap();
        c.effects[1].group = Some(g);
        let gr = c.group_mut(g).unwrap();
        gr.name = "Front".into();
        gr.isolate = true;
        gr.blend = GroupBlend::Behind;
        let s = to_json(&c);
        assert!(s.contains("\"version\": 2"));
        let (d, warn) = from_json(&s).unwrap();
        assert!(warn.is_empty());
        assert_eq!(d.groups.len(), 1);
        let dg = &d.groups[0];
        assert_eq!((dg.name.as_str(), dg.isolate, dg.blend), ("Front", true, GroupBlend::Behind));
        assert_ne!(dg.uid, g, "fresh ids on load");
        let members: Vec<_> = d.effects.iter().map(|e| e.group == Some(dg.uid)).collect();
        assert_eq!(members, vec![true, true, false]);
    }

    #[test]
    fn version_1_loads() {
        let s = r#"{"version":1,"effects":[{"effect":"pixelate"},{"effect":"bulge"}]}"#;
        let (c, warn) = from_json(s).unwrap();
        assert!(warn.is_empty());
        assert_eq!(c.effects.len(), 2);
        assert!(c.groups.is_empty() && c.effects.iter().all(|e| e.group.is_none()));
        // a member that names a missing group is simply ungrouped
        let s = r#"{"version":2,"effects":[{"effect":"pixelate","group":7}]}"#;
        assert!(from_json(s).unwrap().0.effects[0].group.is_none());
    }
}
