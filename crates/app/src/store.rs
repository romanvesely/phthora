//! Settings and preset files on disk.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    match std::env::var_os(var) {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => home().join(fallback),
    }
}

pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("phthora")
}

pub fn data_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join("phthora")
}

pub fn preset_dir() -> PathBuf {
    let d = data_dir().join("presets");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub const PRESET_EXT: &str = ".phthora.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub output_dir: Option<PathBuf>,
    pub input_dir: Option<PathBuf>,
    pub background: [f32; 4],
    pub quality: String,
    /// remembered export formats for video and for still images (`Format::id`)
    pub export_video_format: String,
    pub export_still_format: String,
    /// export size, length, frame rate and format follow the input
    pub export_match_input: bool,
    pub flip_x: bool,
    /// encode MP4 on the GPU (VAAPI) when available
    pub hw_encode: bool,
    pub audio: phthora_audio::AudioSettings,
    /// camera device for webcam input
    pub camera: Option<PathBuf>,
    /// mirror the webcam (like a mirror, the default)
    pub flip_webcam: bool,
    /// send the preview to this v4l2loopback device
    pub virtual_cam: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: None,
            input_dir: None,
            background: [0.0, 0.0, 0.0, 1.0],
            quality: "high".into(),
            export_video_format: "mp4".into(),
            export_still_format: "png".into(),
            export_match_input: false,
            flip_x: false,
            hw_encode: true,
            audio: Default::default(),
            camera: None,
            flip_webcam: true,
            virtual_cam: None,
        }
    }
}

impl Settings {
    fn path() -> PathBuf {
        config_dir().join("settings.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(config_dir());
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::path(), s);
        }
    }

    /// Where exports go by default: the setting, else ~/Videos, else home.
    pub fn export_dir(&self) -> PathBuf {
        if let Some(d) = &self.output_dir
            && d.is_dir() {
                return d.clone();
            }
        let v = home().join("Videos");
        if v.is_dir() { v } else { home() }
    }
}

/// Preset files (`*.phthora.json`) in the preset folder and one level of subfolders, sorted by name.
pub fn list_presets() -> Vec<PathBuf> {
    let root = preset_dir();
    let mut out = Vec::new();
    let scan = |dir: &Path, out: &mut Vec<PathBuf>| {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() && p.to_string_lossy().ends_with(PRESET_EXT) {
                    out.push(p);
                }
            }
        }
    };
    scan(&root, &mut out);
    if let Ok(rd) = std::fs::read_dir(&root) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                scan(&e.path(), &mut out);
            }
        }
    }
    out.sort_by_key(|p| preset_label(p).to_lowercase());
    out
}

/// "folder/name" or "name" for display.
pub fn preset_label(p: &Path) -> String {
    let name = p.file_name().map(|n| n.to_string_lossy().trim_end_matches(PRESET_EXT).to_string()).unwrap_or_default();
    let root = preset_dir();
    match p.parent() {
        Some(par) if par != root => format!("{}/{}", par.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), name),
        _ => name,
    }
}

/// Allowed preset names: letters, digits, space, '-' and '_', at most 24 characters.
pub fn sanitize_name(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ' ')).take(24).collect::<String>().trim().to_string()
}

pub fn preset_path(name: &str) -> PathBuf {
    preset_dir().join(format!("{name}{PRESET_EXT}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_settings_files_get_export_defaults() {
        // a settings file from before the export formats were remembered
        let s: Settings = serde_json::from_str(r#"{"quality":"low","flip_x":true}"#).unwrap();
        assert_eq!((s.quality.as_str(), s.export_video_format.as_str(), s.export_still_format.as_str(), s.export_match_input), ("low", "mp4", "png", false));
        let s2: Settings = serde_json::from_str(&serde_json::to_string(&Settings { export_video_format: "webp".into(), export_match_input: true, ..Settings::default() }).unwrap()).unwrap();
        assert_eq!((s2.export_video_format.as_str(), s2.export_match_input), ("webp", true));
    }

    #[test]
    fn names() {
        assert_eq!(sanitize_name("  my/preset!*  "), "mypreset");
        assert_eq!(sanitize_name("abcdefghijklmnopqrstuvwxyz"), "abcdefghijklmnopqrstuvwx");
    }
}
