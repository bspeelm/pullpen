//! Instants as unix seconds, and the compact age the list shows.
//!
//! Seconds are enough: forges report to the second, and nothing here does
//! calendar arithmetic, so a date library would be weight without a use.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
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

#[cfg(test)]
mod tests;
