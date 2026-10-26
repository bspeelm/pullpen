use std::io::Cursor;

use super::*;
use crate::config::ForgeCfg;

fn forge(kind: Kind) -> ForgeCfg {
    ForgeCfg { kind, host: None, token_cmd: None, scope: None, groups: vec![] }
}

#[test]
fn every_listed_setting_reads_and_takes_its_own_value_back() {
    let mut c = Config { forge: vec![forge(Kind::Github), forge(Kind::Gitlab)], ..Config::default() };
    for (k, _, _) in KEYS {
        let v = get(&c, k).unwrap();
        set(&mut c, k, &v).unwrap_or_else(|e| panic!("{k} refuses its own value {v:?}: {e}"));
        assert_eq!(get(&c, k).unwrap(), v, "{k}");
    }
}

#[test]
fn bad_values_are_refused_with_what_is_allowed() {
    let mut c = Config::default();
    assert!(set(&mut c, "merge.default_method", "fast").unwrap_err().contains("repo, squash"));
    assert!(set(&mut c, "ui.accent", "teal").unwrap_err().contains("#rrggbb"));
    assert!(set(&mut c, "sync.interval", "soon").is_err());
    assert!(set(&mut c, "github.scope", "acme").unwrap_err().contains("pullpen setup"));
    assert!(get(&c, "nope").is_err());
}

#[test]
fn github_scope_takes_bare_names() {
    let mut c = Config { forge: vec![forge(Kind::Github)], ..Config::default() };
    set(&mut c, "github.scope", "acme platform/runner user:me").unwrap();
    assert_eq!(get(&c, "github.scope").unwrap(), "org:acme repo:platform/runner user:me");
}

#[test]
fn the_menu_saves_each_change_as_it_is_made() {
    let mut input = Cursor::new(b"1\nsquash\n7\nsideways\nq\n".to_vec());
    let mut out = Vec::new();
    let mut secret = || Ok(String::new());
    let mut io = Io { input: &mut input, out: &mut out, secret: &mut secret };
    let mut c = Config::default();
    let mut saves = 0;
    let leave = menu(&mut io, &mut c, "github.com", &mut |_| {
        saves += 1;
        Ok(())
    })
    .unwrap();
    assert!(matches!(leave, Leave::Done));
    assert_eq!((c.merge.default_method.as_str(), saves), ("squash", 1), "the refused value saved nothing");
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("✓ merge.default_method = squash") && out.contains("✗ \"sideways\" is not one of"));
}
