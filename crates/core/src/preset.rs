//! Preset files (`.phthora.json`).

use crate::chain::EffectInstance;
use crate::modulator::Modulator;
use crate::params::Value;
use crate::registry;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const EXTENSION: &str = "phthora.json";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PresetFile {
    pub version: u32,
    pub effects: Vec<PresetEffect>,
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
}

fn yes() -> bool {
    true
}

pub fn to_preset(effects: &[EffectInstance]) -> PresetFile {
    PresetFile {
        version: 1,
        effects: effects
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
            })
            .collect(),
    }
}

/// Unknown effects / params are skipped; returns the instances and a list of warnings.
pub fn from_preset(p: &PresetFile) -> (Vec<EffectInstance>, Vec<String>) {
    let mut out = Vec::new();
    let mut warn = Vec::new();
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
        out.push(e);
    }
    (out, warn)
}

pub fn to_json(effects: &[EffectInstance]) -> String {
    serde_json::to_string_pretty(&to_preset(effects)).expect("serializable")
}

pub fn from_json(s: &str) -> Result<(Vec<EffectInstance>, Vec<String>), serde_json::Error> {
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
        let s = to_json(&c.effects);
        let (back, warn) = from_json(&s).unwrap();
        assert!(warn.is_empty(), "{warn:?}");
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
        assert_eq!(a[0].seed, b[0].seed);
        assert_eq!(a[0].modulators["amount"].seed, b[0].modulators["amount"].seed);
        // disabled modulators survive a save
        let (c, _) = from_json(&to_json(&a)).unwrap();
        assert_eq!(c[0].modulators["amount"].shape, crate::modulator::Shape::Noise);
    }

    #[test]
    fn unavailable_effects_are_skipped() {
        let s = r#"{"version":1,"effects":[{"effect":"definitely_not_an_effect"},{"effect":"pixelate"}]}"#;
        let (fx, warn) = from_json(s).unwrap();
        assert_eq!(fx.len(), 1);
        assert_eq!(warn.len(), 1);
    }
}
