use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// `<stem up to 30 chars>-phthora-YYYYMMDD-HHMMSS.<ext>` in local time
/// (UTC where the local offset is unavailable). An empty `ext` gives a name without
/// extension, e.g. for the folder of a PNG sequence.
pub fn default_export_name(input_stem: &str, ext: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    export_name_at(input_stem, ext, now + local_offset_secs(now))
}

/// [`default_export_name`] inside `dir` as a path that is free: not an existing file or folder and not in
/// `taken` (names handed out earlier in the same batch). Where the name is in use, `-2`, `-3`, ... is added
/// after the time stamp.
pub fn unique_export_path(dir: &Path, input_stem: &str, ext: &str, taken: &HashSet<PathBuf>) -> PathBuf {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    unique_export_path_at(dir, input_stem, ext, now + local_offset_secs(now), taken)
}

/// Same as [`unique_export_path`] for an explicit wall-clock time (see [`export_name_at`]).
pub fn unique_export_path_at(dir: &Path, input_stem: &str, ext: &str, local_unix_secs: i64, taken: &HashSet<PathBuf>) -> PathBuf {
    let base = export_name_at(input_stem, "", local_unix_secs);
    let ext = ext.trim_start_matches('.');
    let dot = if ext.is_empty() { "" } else { "." };
    (1u32..)
        .map(|n| {
            let number = if n == 1 { String::new() } else { format!("-{n}") };
            dir.join(format!("{base}{number}{dot}{ext}"))
        })
        .find(|p| !taken.contains(p) && !p.exists())
        .expect("an unused number exists")
}

/// Same as [`default_export_name`] for an explicit wall-clock time, given as
/// Unix seconds already shifted into the desired time zone.
pub fn export_name_at(input_stem: &str, ext: &str, local_unix_secs: i64) -> String {
    let stem: String = input_stem
        .chars()
        .map(|c| if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
        .take(30)
        .collect();
    let stem = stem.trim();
    let stem = if stem.is_empty() { "output" } else { stem };
    let (y, mo, d, h, mi, s) = civil(local_unix_secs);
    let ext = ext.trim_start_matches('.');
    let dot = if ext.is_empty() { "" } else { "." };
    format!("{stem}-phthora-{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}{dot}{ext}")
}

/// Unix seconds -> (year, month, day, hour, minute, second), proleptic Gregorian.
fn civil(t: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = t.div_euclid(86400);
    let rem = t.rem_euclid(86400) as u32;
    // Howard Hinnant's days_from_civil inverse.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + (m <= 2) as i64;
    (y, m, d, rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Offset of the local time zone from UTC at `t`, in seconds.
fn local_offset_secs(t: i64) -> i64 {
    use chrono::{Offset, TimeZone};
    chrono::Local.timestamp_opt(t, 0).single().map_or(0, |d| d.offset().fix().local_minus_utc() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil(1_000_000_000), (2001, 9, 9, 1, 46, 40));
        assert_eq!(civil(1_709_164_800 + 86399), (2024, 2, 29, 23, 59, 59));
        assert_eq!(civil(-1), (1969, 12, 31, 23, 59, 59));
    }

    #[test]
    fn name_format() {
        let n = export_name_at("my/clip", "mp4", 1_000_000_000);
        assert_eq!(n, "my_clip-phthora-20010909-014640.mp4");
        let long = export_name_at(&"x".repeat(50), ".gif", 0);
        assert!(long.starts_with(&format!("{}-phthora-", "x".repeat(30))));
        assert!(long.ends_with(".gif"));
        assert!(export_name_at("", "png", 0).starts_with("output-phthora-"));
        assert_eq!(export_name_at("clip", "", 1_000_000_000), "clip-phthora-20010909-014640");
        let now = default_export_name("a", "webm");
        assert!(now.starts_with("a-phthora-") && now.ends_with(".webm"));
    }

    #[test]
    fn unique_paths_never_reuse_a_name() {
        let dir = std::env::temp_dir().join(format!("phthora-name-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let t = 1_000_000_000;
        let mut taken = HashSet::new();
        let a = unique_export_path_at(&dir, "clip", "mp4", t, &taken);
        assert_eq!(a, dir.join("clip-phthora-20010909-014640.mp4"));
        // handed out earlier in the batch
        taken.insert(a.clone());
        let b = unique_export_path_at(&dir, "clip", "mp4", t, &taken);
        assert_eq!(b, dir.join("clip-phthora-20010909-014640-2.mp4"));
        // already on disk
        std::fs::write(&b, b"x").unwrap();
        let c = unique_export_path_at(&dir, "clip", "mp4", t, &taken);
        assert_eq!(c, dir.join("clip-phthora-20010909-014640-3.mp4"));
        // a long stem keeps the number (it comes after the time stamp), a folder has no extension
        let long = "x".repeat(50);
        taken.insert(unique_export_path_at(&dir, &long, "", t, &taken));
        let f = unique_export_path_at(&dir, &long, "", t, &taken);
        assert!(f.file_name().unwrap().to_string_lossy().ends_with("20010909-014640-2"), "{f:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
