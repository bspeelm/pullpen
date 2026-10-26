//! Sentences: how a verdict or a fact is said. The facts themselves come
//! from `pullpen_core::verdict`; only wording lives here.

use pullpen_core::{Block, Pr, Verdict, Wait};

fn people(pending: &[String]) -> String {
    let at: Vec<String> = pending.iter().map(|p| format!("@{p}")).collect();
    match at.len() {
        0 => String::new(),
        1 => at[0].clone(),
        _ => format!("{} and {}", at[..at.len() - 1].join(", "), at[at.len() - 1]),
    }
}

fn reviews_left(have: u32, need: u32) -> String {
    let n = need.saturating_sub(have);
    if n == 1 { "1 review".into() } else { format!("{n} reviews") }
}

/// What a single wait is for, as a clause: "CI (4/9)", "1 review".
fn wait_clause(w: &Wait) -> String {
    match w {
        Wait::Ci { passed, total } => format!("CI ({passed}/{total})"),
        Wait::Reviews { have, need, .. } => reviews_left(*have, *need),
        Wait::Train { position, length } => format!("the merge train ({position} of {length})"),
        Wait::Behind { base } => format!("an update from {base}"),
        Wait::Mergeability => "the forge's mergeability check".into(),
    }
}

fn join(clauses: Vec<String>) -> String {
    match clauses.len() {
        0 => String::new(),
        1 => clauses[0].clone(),
        _ => format!("{} and {}", clauses[..clauses.len() - 1].join(", "), clauses[clauses.len() - 1]),
    }
}

/// When an armed PR will merge, as a condition: "when @mara approves".
fn armed_condition(waits: &[Wait]) -> String {
    match waits {
        [Wait::Reviews { pending, .. }] if pending.len() == 1 => {
            format!("when {} approves", people(pending))
        }
        [Wait::Reviews { pending, .. }] if !pending.is_empty() => {
            format!("when {} approve", people(pending))
        }
        [Wait::Ci { .. }] => "when CI passes".into(),
        _ => format!("after {}", join(waits.iter().map(wait_clause).collect())),
    }
}

fn method(pr: &Pr) -> &'static str {
    pr.auto_merge.as_ref().map_or(pr.repo_default_method, |a| a.method).word()
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

/// The preview's verdict clause, after `○ waiting` and friends.
pub fn verdict_short(pr: &Pr, v: &Verdict) -> String {
    match v {
        Verdict::Ready => format!("M merges with {}", method(pr)),
        Verdict::Waiting(w) if w.iter().all(|x| matches!(x, Wait::Behind { .. })) => {
            format!("behind {}: u updates it", pr.base)
        }
        Verdict::Waiting(w) if pr.armed() => {
            format!("{}-merges {}", method(pr), armed_condition(w))
        }
        Verdict::Waiting(w) => format!("waiting on {}", join(w.iter().map(wait_clause).collect())),
        Verdict::Blocked(b) => block_clause(b),
    }
}

fn block_clause(b: &Block) -> String {
    match b {
        Block::Draft => "draft".into(),
        Block::Conflict { base } => format!("conflicts with {base}"),
        Block::Failing { check } => format!("{check} failing"),
        Block::ChangesRequested => "changes requested".into(),
    }
}

/// The detail view's verdict sentence (PLAN.md §6.4).
pub fn verdict_long(pr: &Pr, v: &Verdict) -> String {
    match v {
        Verdict::Ready => format!("Nothing blocking. M merges with {}.", method(pr)),
        Verdict::Waiting(w) if w.iter().all(|x| matches!(x, Wait::Behind { .. })) => {
            format!("Behind {}. u updates the branch.", pr.base)
        }
        Verdict::Waiting(w) if pr.armed() => {
            format!("{}-merges automatically {}. Nothing else is blocking.", capital(method(pr)), armed_condition(w))
        }
        Verdict::Waiting(w) => format!("Waiting on {}. A arms auto-merge.", join(w.iter().map(wait_clause).collect())),
        Verdict::Blocked(Block::Failing { check }) => {
            format!("{check} failing. d opens the diff, o the run.")
        }
        Verdict::Blocked(Block::Conflict { base }) => {
            format!("Conflicts with {base}. :checkout to resolve.")
        }
        Verdict::Blocked(Block::ChangesRequested) => "Changes requested. o opens the review.".into(),
        Verdict::Blocked(Block::Draft) => "Draft. Mark it ready on the forge first; o opens it.".into(),
    }
}

#[cfg(test)]
mod tests;
