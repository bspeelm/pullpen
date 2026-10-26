use std::collections::HashMap;
use std::path::PathBuf;

use super::Dirs;

fn resolve(vars: &[(&str, &str)]) -> anyhow::Result<Dirs> {
    let m: HashMap<String, PathBuf> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), PathBuf::from(v)))
        .collect();
    Dirs::resolve(|k| m.get(k).cloned())
}

#[test]
fn falls_back_to_home_when_xdg_is_unset() {
    let d = resolve(&[("HOME", "/h")]).unwrap();
    assert_eq!(d.config, PathBuf::from("/h/.config/pullpen"));
    assert_eq!(d.cache, PathBuf::from("/h/.cache/pullpen"));
    assert_eq!(d.state, PathBuf::from("/h/.local/state/pullpen"));
}

#[test]
fn honours_xdg_variables() {
    let d = resolve(&[
        ("HOME", "/h"),
        ("XDG_CONFIG_HOME", "/c"),
        ("XDG_CACHE_HOME", "/k"),
        ("XDG_STATE_HOME", "/s"),
    ])
    .unwrap();
    assert_eq!(d.config, PathBuf::from("/c/pullpen"));
    assert_eq!(d.cache, PathBuf::from("/k/pullpen"));
    assert_eq!(d.state, PathBuf::from("/s/pullpen"));
}

#[test]
fn a_relative_xdg_value_is_ignored_not_joined_to_the_cwd() {
    let d = resolve(&[("HOME", "/h"), ("XDG_STATE_HOME", "rel/state")]).unwrap();
    assert_eq!(d.state, PathBuf::from("/h/.local/state/pullpen"));
}

#[test]
fn refuses_to_guess_without_a_home() {
    assert!(resolve(&[]).is_err());
    assert!(resolve(&[("HOME", "relative")]).is_err());
}
