use crate::time::{Timestamp, age, parse_age, parse_rfc3339, rfc3339};

#[test]
fn ages_round_down_to_the_largest_unit() {
    let now = Timestamp(10_000_000);
    let ago = |s| age(Timestamp(now.0 - s), now);
    assert_eq!(ago(30), "0m");
    assert_eq!(ago(12 * 60), "12m");
    assert_eq!(ago(59 * 60 + 59), "59m");
    assert_eq!(ago(3600), "1h");
    assert_eq!(ago(23 * 3600), "23h");
    assert_eq!(ago(2 * 86_400), "2d");
    assert_eq!(ago(13 * 86_400), "1w");
    assert_eq!(ago(15 * 86_400), "2w");
}

#[test]
fn a_clock_behind_the_forge_never_shows_a_negative_age() {
    assert_eq!(age(Timestamp(100), Timestamp(50)), "0m");
}

#[test]
fn every_design_age_round_trips() {
    for s in [
        "41m", "3h", "5h", "12m", "1h", "1d", "2h", "6h", "30m", "2d", "3d", "4h", "4d", "7h", "5d", "6d", "1w", "2w",
        "20m", "8h",
    ] {
        let now = Timestamp(100_000_000);
        assert_eq!(age(Timestamp(now.0 - parse_age(s).unwrap()), now), s);
    }
}

#[test]
fn rfc3339_in_the_shapes_both_forges_send() {
    assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(Timestamp(0)));
    assert_eq!(parse_rfc3339("2026-10-25T19:28:04Z"), Some(Timestamp(1_792_956_484)));
    assert_eq!(parse_rfc3339("2026-10-25T19:28:04.123Z"), Some(Timestamp(1_792_956_484)));
    assert_eq!(parse_rfc3339("2026-10-25T21:28:04+02:00"), Some(Timestamp(1_792_956_484)));
    assert_eq!(parse_rfc3339("2000-02-29T00:00:00Z"), Some(Timestamp(951_782_400)));
    assert_eq!(parse_rfc3339("yesterday"), None);
}

#[test]
fn rfc3339_round_trips_including_leap_days() {
    for t in ["2026-10-25T19:28:04Z", "2024-02-29T00:00:00Z", "1999-12-31T23:59:59Z", "2000-03-01T12:00:00Z"] {
        assert_eq!(rfc3339(parse_rfc3339(t).unwrap()), t);
    }
}
