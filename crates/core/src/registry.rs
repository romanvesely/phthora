//! Built-in effect descriptors.

use crate::params::ParamDef;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind {
    /// A single fragment shader on a fullscreen triangle.
    Shader,
    /// Multi-pass, history, geometry or CPU-assisted effect.
    Custom,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EffectDesc {
    pub id: String,
    pub name: String,
    pub category: String,
    pub kind: EffectKind,
    pub shader: Option<String>,
    pub randomizable: bool,
    /// Mask effects cut the effect directly above them.
    pub mask: bool,
    /// Uses the `time` uniform (animated); gets an image-mode seed param.
    pub time: bool,
    pub params: Vec<ParamDef>,
}

impl EffectDesc {
    pub fn param(&self, key: &str) -> Option<&ParamDef> {
        self.params.iter().find(|p| p.key == key)
    }
}

static REGISTRY: OnceLock<Vec<EffectDesc>> = OnceLock::new();

/// All effects in default order.
pub fn registry() -> &'static [EffectDesc] {
    REGISTRY.get_or_init(|| {
        serde_json::from_str(include_str!("../data/effects.json")).expect("effects.json is valid")
    })
}

/// Effects described in the catalog but not implemented yet.
pub const NOT_AVAILABLE: &[&str] = &[];

impl EffectDesc {
    /// Whether the effect can be added from the UI and the command line.
    pub fn available(&self) -> bool {
        !NOT_AVAILABLE.contains(&self.id.as_str())
    }
}

/// Effects that can be added, in catalog order.
pub fn available() -> impl Iterator<Item = &'static EffectDesc> {
    registry().iter().filter(|d| d.available())
}

pub fn find(id: &str) -> Option<&'static EffectDesc> {
    registry().iter().find(|e| e.id == id)
}

/// Category names in display order.
pub fn categories() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for e in registry() {
        if !v.contains(&e.category.as_str()) {
            v.push(&e.category);
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_loads() {
        let r = registry();
        assert!(r.len() >= 60);
        let mut ids: Vec<_> = r.iter().map(|e| &e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), r.len(), "duplicate ids");
        for e in r {
            if e.kind == EffectKind::Shader {
                assert!(e.shader.is_some(), "{} has no shader", e.id);
            }
            for p in &e.params {
                let _ = p.default_value();
            }
        }
        assert!(find("bad_tv").unwrap().time);
    }
}
