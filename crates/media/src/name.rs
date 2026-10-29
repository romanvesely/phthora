use std::time::{SystemTime, UNIX_EPOCH};

/// `<stem up to 30 chars>-phthora-YYYYMMDD-HHMMSS.<ext>` in local time
/// (UTC where the local offset is unavailable).
pub fn default_export_name(input_stem: &str, ext: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    export_name_at(input_stem, ext, now + local_offset_secs(now))
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
    format!("{stem}-phthora-{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}.{}", ext.trim_start_matches('.'))
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

#[cfg(unix)]
fn local_offset_secs(t: i64) -> i64 {
    use std::os::raw::{c_char, c_int, c_long};
    #[repr(C)]
    struct Tm {
        tm_sec: c_int,
        tm_min: c_int,
        tm_hour: c_int,
        tm_mday: c_int,
        tm_mon: c_int,
        tm_year: c_int,
        tm_wday: c_int,
        tm_yday: c_int,
        tm_isdst: c_int,
        tm_gmtoff: c_long,
        tm_zone: *const c_char,
    }
    unsafe extern "C" {
        fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    }
    // SAFETY: Tm matches the glibc/musl/BSD `struct tm` layout; localtime_r
    // only writes into the buffer we pass.
    unsafe {
        let mut tm: Tm = std::mem::zeroed();
        if localtime_r(&t, &mut tm).is_null() {
            0
        } else {
            tm.tm_gmtoff as i64
        }
    }
}

#[cfg(not(unix))]
fn local_offset_secs(_t: i64) -> i64 {
    0
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
        let now = default_export_name("a", "webm");
        assert!(now.starts_with("a-phthora-") && now.ends_with(".webm"));
    }
}
