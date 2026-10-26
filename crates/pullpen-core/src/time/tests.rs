use crate::time::{Timestamp, age, parse_age};

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
