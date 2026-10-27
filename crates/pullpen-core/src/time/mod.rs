//! Instants as unix seconds, and the compact age the list shows.
//!
//! Seconds are enough: forges report to the second, and nothing here does
//! calendar arithmetic, so a date library would be weight without a use.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct Timestamp(pub i64);

impl Timestamp {
    pub fn now() -> Self {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        Timestamp(secs)
    }
}

/// `12m 3h 2d 1w`: at most three cells, so it right-aligns in a four-cell
/// column (design §10, "durations are compact"). Anything under a minute
/// reads as `0m` rather than a seconds unit nobody needs here.
pub fn age(then: Timestamp, now: Timestamp) -> String {
    let s = (now.0 - then.0).max(0);
    let m = s / 60;
    match () {
        _ if m < 60 => format!("{m}m"),
        _ if m < 24 * 60 => format!("{}h", m / 60),
        _ if m < 7 * 24 * 60 => format!("{}d", m / (24 * 60)),
        _ => format!("{}w", m / (7 * 24 * 60)),
    }
}

/// Parses the compact form back into seconds. Fixtures and tests are written
/// in it, because that is how the design states every age.
pub fn parse_age(s: &str) -> Option<i64> {
    let (n, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: i64 = n.parse().ok()?;
    let mult = match unit {
        "m" => 60,
        "h" => 3600,
        "d" => 86_400,
        "w" => 604_800,
        _ => return None,
    };
    Some(n * mult)
}

/// `2026-10-25T19:28:04Z`, with optional fractional seconds and a numeric
/// offset, as both forges write them.
pub fn parse_rfc3339(s: &str) -> Option<Timestamp> {
    let b = s.as_bytes();
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || !matches!(b[10], b'T' | b't' | b' ') {
        return None;
    }
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, sec) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let mut i = 19;
    if b.get(i) == Some(&b'.') {
        i += 1;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
    }
    let offset = match b.get(i)? {
        b'Z' | b'z' => 0,
        sign @ (b'+' | b'-') => {
            let o = num(i + 1..i + 3)? * 3600 + num(i + 4..i + 6)? * 60;
            if *sign == b'+' { o } else { -o }
        }
        _ => return None,
    };
    Some(Timestamp(days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + sec - offset))
}

/// The inverse of `parse_rfc3339`, in UTC: what forge searches take.
pub fn rfc3339(t: Timestamp) -> String {
    let (days, secs) = (t.0.div_euclid(86_400), t.0.rem_euclid(86_400));
    // Howard Hinnant's `civil_from_days`.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", secs / 3600, secs % 3600 / 60, secs % 60)
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `1m 52s`, `38s`, `1h 4m`: how long a check took.
pub fn duration(from: Timestamp, to: Timestamp) -> String {
    let s = (to.0 - from.0).max(0);
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m {:02}s", s / 60, s % 60),
        _ => format!("{}h {}m", s / 3600, s % 3600 / 60),
    }
}

#[cfg(test)]
mod tests;
