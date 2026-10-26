//! Unified diffs, parsed into files and hunks for the diff pane.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Context,
    Added,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLine {
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub kind: Kind,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hunk {
    /// `@@ -38,14 +38,24 @@ migrateInvoices(db, opts)`
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub additions: u32,
    pub deletions: u32,
    pub hunks: Vec<Hunk>,
}

/// `-38,14 +38,24` → (38, 38).
fn starts(header: &str) -> (u32, u32) {
    let mut it = header.split_whitespace().skip(1);
    let num = |t: Option<&str>| t.and_then(|t| t[1..].split(',').next()?.parse().ok()).unwrap_or(1);
    (num(it.next()), num(it.next()))
}

/// Parses `git diff` output. Lines it does not recognise (index lines,
/// mode changes, `\ No newline`) are skipped rather than failing the file.
pub fn parse(text: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let (mut old, mut new) = (0, 0);
    for l in text.lines() {
        if let Some(rest) = l.strip_prefix("diff --git ") {
            let path = rest.rsplit(" b/").next().unwrap_or(rest).to_string();
            files.push(FileDiff { path, additions: 0, deletions: 0, hunks: Vec::new() });
        } else if let Some(p) = l.strip_prefix("+++ b/") {
            if let Some(f) = files.last_mut() {
                f.path = p.to_string();
            }
        } else if l.starts_with("@@") {
            (old, new) = starts(l);
            if let Some(f) = files.last_mut() {
                f.hunks.push(Hunk { header: l.to_string(), lines: Vec::new() });
            }
        } else if let Some(h) = files.last_mut().and_then(|f| f.hunks.last_mut()) {
            let (kind, o, n) = match l.chars().next() {
                Some('+') if !l.starts_with("+++") => (Kind::Added, None, Some(new)),
                Some('-') if !l.starts_with("---") => (Kind::Removed, Some(old), None),
                Some(' ') | None => (Kind::Context, Some(old), Some(new)),
                _ => continue,
            };
            old += u32::from(o.is_some());
            new += u32::from(n.is_some());
            h.lines.push(DiffLine { old: o, new: n, kind, text: l.get(1..).unwrap_or("").to_string() });
        }
    }
    for f in &mut files {
        let lines = f.hunks.iter().flat_map(|h| &h.lines);
        f.additions = lines.clone().filter(|l| l.kind == Kind::Added).count() as u32;
        f.deletions = lines.filter(|l| l.kind == Kind::Removed).count() as u32;
    }
    files
}

#[cfg(test)]
mod tests;
