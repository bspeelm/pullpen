use std::collections::HashMap;
use std::path::PathBuf;

use super::Dirs;

/// An absolute path on whichever platform runs the test: `/h` is not
/// absolute on Windows, which has drive letters.
fn abs(s: &str) -> PathBuf {
    std::env::temp_dir().join(s)
}

fn resolve(vars: &[(&str, PathBuf)], windows: bool) -> anyhow::Result<Dirs> {
    let m: HashMap<String, PathBuf> = vars.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
    Dirs::resolve(|k| m.get(k).cloned(), windows)
}

#[test]
fn falls_back_to_home_when_xdg_is_unset() {
    let d = resolve(&[("HOME", abs("h"))], false).unwrap();
    assert_eq!(d.config, abs("h").join(".config/pullpen"));
    assert_eq!(d.cache, abs("h").join(".cache/pullpen"));
    assert_eq!(d.state, abs("h").join(".local/state/pullpen"));
}

#[test]
fn honours_xdg_variables() {
    let vars =
        [("HOME", abs("h")), ("XDG_CONFIG_HOME", abs("c")), ("XDG_CACHE_HOME", abs("k")), ("XDG_STATE_HOME", abs("s"))];
    let d = resolve(&vars, false).unwrap();
    assert_eq!(d.config, abs("c").join("pullpen"));
    assert_eq!(d.cache, abs("k").join("pullpen"));
    assert_eq!(d.state, abs("s").join("pullpen"));
}

#[test]
fn a_relative_xdg_value_is_ignored_not_joined_to_the_cwd() {
    let d = resolve(&[("HOME", abs("h")), ("XDG_STATE_HOME", "rel/state".into())], false).unwrap();
    assert_eq!(d.state, abs("h").join(".local/state/pullpen"));
}

#[test]
fn refuses_to_guess_without_a_home() {
    assert!(resolve(&[], false).is_err());
    assert!(resolve(&[("HOME", "relative".into())], false).is_err());
}

#[test]
fn windows_uses_appdata_for_settings_and_localappdata_for_the_rest() {
    let d = resolve(&[("APPDATA", abs("Roaming")), ("LOCALAPPDATA", abs("Local"))], true).unwrap();
    assert_eq!(d.config, abs("Roaming").join("pullpen"));
    assert_eq!(d.cache, abs("Local").join("pullpen").join("cache"));
    assert_eq!(d.state, abs("Local").join("pullpen").join("state"));
}

#[test]
fn windows_still_honours_xdg_variables_set_on_purpose() {
    let d = resolve(&[("APPDATA", abs("R")), ("LOCALAPPDATA", abs("L")), ("XDG_CONFIG_HOME", abs("c"))], true).unwrap();
    assert_eq!(d.config, abs("c").join("pullpen"));
    assert_eq!(d.cache, abs("L").join("pullpen").join("cache"));
}

#[test]
fn windows_refuses_to_guess_without_appdata() {
    assert!(resolve(&[("HOME", abs("h"))], true).is_err());
}
