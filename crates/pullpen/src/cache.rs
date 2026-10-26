//! The last sync of each section, so the list paints before the network
//! answers (PLAN.md §1). A cache written for a different query is ignored.

use std::fs;
use std::path::{Path, PathBuf};

use pullpen_core::{Pr, Section};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Entry {
    query: String,
    total: usize,
    prs: Vec<Pr>,
}

fn file(dir: &Path, s: &Section) -> PathBuf {
    let slug: String =
        s.name.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    dir.join("sections").join(format!("{slug}.json"))
}

pub fn load(dir: &Path, s: &Section) -> Option<(Vec<Pr>, usize)> {
    let e: Entry = serde_json::from_str(&fs::read_to_string(file(dir, s)).ok()?).ok()?;
    (e.query == s.query).then_some((e.prs, e.total))
}

/// Written to a temporary file and renamed, so a crash mid-write leaves the
/// previous cache rather than half of one.
pub fn store(dir: &Path, s: &Section, prs: &[Pr], total: usize) -> std::io::Result<()> {
    let path = file(dir, s);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let entry = Entry { query: s.query.clone(), total, prs: prs.to_vec() };
    fs::write(&tmp, serde_json::to_vec(&entry).map_err(std::io::Error::other)?)?;
    fs::rename(tmp, path)
}
