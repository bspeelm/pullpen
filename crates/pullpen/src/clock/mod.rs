//! The local time zone's offset from UTC, for the armed view's clock. The
//! standard library has no time zones and none of the allowed crates does
//! either (ADR-005), so the platform is asked; a clock shown in UTC is the
//! fallback.

use std::process::Command;

/// Seconds east of UTC.
pub fn utc_offset() -> i64 {
    let ask = |bin: &str, args: &[&str]| {
        let o = Command::new(bin).args(args).output().ok();
        o.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
    };
    if cfg!(windows) {
        let key = r"HKLM\SYSTEM\CurrentControlSet\Control\TimeZoneInformation";
        from_bias(&ask("reg", &["query", key, "/v", "ActiveTimeBias"]))
    } else {
        from_z(&ask("date", &["+%z"]))
    }
}

/// `date +%z` prints `+0200`.
fn from_z(z: &str) -> i64 {
    let (sign, digits) = match z.trim().split_at_checked(1) {
        Some(("-", d)) => (-1, d),
        Some(("+", d)) => (1, d),
        _ => return 0,
    };
    let n: i64 = digits.parse().unwrap_or(0);
    sign * (n / 100 * 3600 + n % 100 * 60)
}

/// `reg query` prints `ActiveTimeBias    REG_DWORD    0xfffffe20`: minutes
/// west of UTC, daylight saving included, as a 32-bit two's-complement word.
fn from_bias(out: &str) -> i64 {
    let word = out.lines().find(|l| l.contains("ActiveTimeBias")).and_then(|l| l.split_whitespace().last());
    let bias = word.and_then(|w| u32::from_str_radix(w.trim_start_matches("0x"), 16).ok()).unwrap_or(0) as i32;
    -i64::from(bias) * 60
}

#[cfg(test)]
mod tests;
