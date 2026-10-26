use super::*;

/// `make keymap` runs this with PULLPEN_BLESS set, which writes the file
/// instead of comparing.
#[test]
fn docs_keymap_md_is_generated_from_the_table() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/keymap.md");
    if std::env::var_os("PULLPEN_BLESS").is_some() {
        std::fs::write(&path, markdown()).unwrap();
    }
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(on_disk == markdown(), "docs/keymap.md is stale; run `make keymap`");
}

#[test]
fn section_keys_follow_the_section_count() {
    let e = ENTRIES.iter().find(|e| e.keys == "1–N").unwrap();
    assert_eq!(keys_for(e, 5), "1–5");
    assert_eq!(keys_for(e, 12), "1–9");
}

#[test]
fn remaps_read_names_with_or_without_a_view_prefix() {
    let cfg = [
        ("list.merge".to_string(), "m".to_string()),
        ("quit".to_string(), "^q".to_string()),
        ("nope".to_string(), "z".to_string()),
    ];
    let (ok, bad) = remaps(cfg.iter().map(|(a, b)| (a, b)));
    assert_eq!(ok, vec![(Key::Char('m'), Key::Char('M')), (Key::Ctrl('q'), Key::Char('q'))]);
    assert_eq!(bad, vec!["[keys] nope: no such action".to_string()]);
}
