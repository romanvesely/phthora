//! Parameter definitions and values.

use serde::{Deserialize, Serialize};

/// Range used when randomizing a float parameter ("chaos").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RandRange {
    #[default]
    Full,
    None,
    Low,
    Mid,
    High,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorStop {
    pub color: String,
    pub stop: f32,
    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParamKind {
    Float {
        min: f32,
        max: f32,
        step: f32,
        default: f32,
        #[serde(default)]
        rand: RandRange,
        #[serde(default)]
        no_random: bool,
        #[serde(default)]
        disable_when: Option<String>,
        /// Only shown in image mode (the per-effect seed).
        #[serde(default)]
        image_only: bool,
    },
    Bool {
        default: bool,
    },
    Combo {
        values: Vec<String>,
        default: String,
        #[serde(default)]
        no_random: bool,
    },
    Color {
        default: String,
    },
    ColorStops {
        default: Vec<ColorStop>,
    },
    Button,
    Text {
        #[serde(default)]
        multiline: bool,
        default: String,
    },
    Info {
        #[serde(default)]
        default: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParamDef {
    pub key: String,
    pub label: String,
    #[serde(flatten)]
    pub kind: ParamKind,
    /// Stored in presets but not shown in the UI.
    #[serde(default)]
    pub hidden: bool,
}

impl ParamDef {
    pub fn default_value(&self) -> Option<Value> {
        Some(match &self.kind {
            ParamKind::Float { default, .. } => Value::Float(*default),
            ParamKind::Bool { default } => Value::Bool(*default),
            ParamKind::Combo { default, .. } => Value::Text(default.clone()),
            ParamKind::Color { default } => Value::Color(parse_hex(default)),
            ParamKind::ColorStops { default } => Value::Stops(default.clone()),
            ParamKind::Text { default, .. } | ParamKind::Info { default } => Value::Text(default.clone()),
            ParamKind::Button => return None,
        })
    }

    pub fn is_float(&self) -> bool {
        matches!(self.kind, ParamKind::Float { .. })
    }

    /// (min, max) for float params.
    pub fn range(&self) -> Option<(f32, f32)> {
        match self.kind {
            ParamKind::Float { min, max, .. } => Some((min, max)),
            _ => None,
        }
    }

    /// Lower/upper bound used when randomizing.
    pub fn rand_bounds(&self) -> Option<(f32, f32)> {
        let ParamKind::Float { min, max, default, rand, .. } = self.kind else { return None };
        let range = max - min;
        Some(match rand {
            RandRange::None => (default, default),
            RandRange::High => (min + range * 0.75, max),
            RandRange::Low => (min, min + range * 0.4),
            // the reference ignores `min` here; keep that behavior
            RandRange::Mid => (range / 2.0 - range / 4.0, range / 2.0 + range / 4.0),
            RandRange::Full => (min, max),
        })
    }
}

/// A parameter value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Float(f32),
    Bool(bool),
    Text(String),
    /// RGB 0..1 (no color management, raw hex/255).
    Color([f32; 3]),
    Stops(Vec<ColorStop>),
}

impl Value {
    pub fn as_f32(&self) -> f32 {
        match self {
            Value::Float(v) => *v,
            Value::Bool(b) => *b as i32 as f32,
            _ => 0.0,
        }
    }
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Float(v) => *v > 0.5,
            _ => false,
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            Value::Text(s) => s,
            _ => "",
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Float(v) => serde_json::json!(v),
            Value::Bool(v) => serde_json::json!(v),
            Value::Text(v) => serde_json::json!(v),
            Value::Color(c) => serde_json::json!(to_hex(*c)),
            Value::Stops(s) => serde_json::to_value(s).unwrap_or_default(),
        }
    }

    pub fn from_json(def: &ParamDef, v: &serde_json::Value) -> Option<Value> {
        Some(match &def.kind {
            ParamKind::Float { .. } => Value::Float(match v {
                serde_json::Value::String(s) => s.parse().ok()?,
                _ => v.as_f64()? as f32,
            }),
            ParamKind::Bool { .. } => Value::Bool(v.as_bool()?),
            ParamKind::Combo { values, .. } => {
                let s = v.as_str()?;
                if !values.iter().any(|x| x == s) {
                    return None;
                }
                Value::Text(s.to_string())
            }
            ParamKind::Color { .. } => Value::Color(parse_hex(v.as_str()?)),
            ParamKind::ColorStops { .. } => Value::Stops(serde_json::from_value(v.clone()).ok()?),
            ParamKind::Text { .. } | ParamKind::Info { .. } => Value::Text(v.as_str()?.to_string()),
            ParamKind::Button => return None,
        })
    }
}

pub fn parse_hex(s: &str) -> [f32; 3] {
    let h = s.trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).unwrap_or(0xffffff);
    let (n, len) = if h.len() == 3 {
        let r = (n >> 8) & 0xf;
        let g = (n >> 4) & 0xf;
        let b = n & 0xf;
        ((r * 17) << 16 | (g * 17) << 8 | (b * 17), 6)
    } else {
        (n, h.len())
    };
    let _ = len;
    [
        ((n >> 16) & 0xff) as f32 / 255.0,
        ((n >> 8) & 0xff) as f32 / 255.0,
        (n & 0xff) as f32 / 255.0,
    ]
}

pub fn to_hex(c: [f32; 3]) -> String {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hex_roundtrip() {
        assert_eq!(to_hex(parse_hex("#46ff03")), "#46ff03");
        assert_eq!(to_hex(parse_hex("fff")), "#ffffff");
    }
}
