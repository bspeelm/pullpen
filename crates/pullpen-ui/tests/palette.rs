//! ADR-003: the only colour values in this repository are the built-in
//! palette's. The check is inverted — it fails on any `#rrggbb` not in the
//! palette — so it names nothing it forbids and catches any stray palette.

// A test that cannot read its inputs should fail loudly, which is what
// unwrap does; clippy's test allowance does not reach helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use pullpen_ui::palette::{Tok, hex};

// design/ is the palette's source (ADR-003); target/ and .git are not ours.
const SKIP: &[&str] = &["target", ".git", "design"];

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name();
        if SKIP.iter().any(|s| name == *s) {
            continue;
        }
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn colours(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut found = Vec::new();
    for i in 0..b.len() {
        if b[i] != b'#' {
            continue;
        }
        let run = b[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_hexdigit())
            .count();
        // Exactly six hex digits, and at least one a–f, so `#123456` as an
        // issue number is not mistaken for a colour.
        let digits = &text[i + 1..i + 1 + run];
        if run == 6 && digits.bytes().any(|c| c.is_ascii_alphabetic()) {
            found.push(format!("#{}", digits.to_ascii_lowercase()));
        }
    }
    found
}

#[test]
fn only_the_built_in_palette_is_shipped() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let allowed: BTreeSet<String> = Tok::ALL.iter().map(|t| hex(*t)).collect();
    let mut files = Vec::new();
    walk(&root, &mut files);
    assert!(
        files.len() > 10,
        "walked only {} files from {}",
        files.len(),
        root.display()
    );
    let mut stray = Vec::new();
    for f in files {
        // Binary files cannot hold a colour a reader would see.
        let Ok(text) = fs::read_to_string(&f) else {
            continue;
        };
        for c in colours(&text) {
            if !allowed.contains(&c) {
                stray.push(format!(
                    "{}: {c}",
                    f.strip_prefix(&root).unwrap_or(&f).display()
                ));
            }
        }
    }
    assert!(
        stray.is_empty(),
        "colour values outside the built-in palette (ADR-003):\n{}",
        stray.join("\n")
    );
}

#[test]
fn the_scanner_sees_a_colour() {
    assert_eq!(
        colours("accent = \"#7CC4DC\" and #4821 and #123456"),
        vec!["#7cc4dc"]
    );
}
