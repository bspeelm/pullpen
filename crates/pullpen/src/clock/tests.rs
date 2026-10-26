use super::{from_bias, from_z};

#[test]
fn date_offsets_east_and_west() {
    assert_eq!(from_z("+0200\n"), 7200);
    assert_eq!(from_z("-0530"), -(5 * 3600 + 30 * 60));
    assert_eq!(from_z(""), 0, "no date command is UTC");
}

#[test]
fn windows_bias_is_minutes_west_as_an_unsigned_word() {
    let reg = "\r\nHKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\TimeZoneInformation\r\n    ActiveTimeBias    REG_DWORD    0xfffffe20\r\n\r\n";
    assert_eq!(from_bias(reg), 8 * 3600, "-480 minutes west is UTC+8");
    assert_eq!(from_bias("    ActiveTimeBias    REG_DWORD    0x12c"), -5 * 3600, "300 west is UTC-5");
    assert_eq!(from_bias(""), 0);
}
