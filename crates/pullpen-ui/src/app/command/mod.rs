//! The `:` commands: the table, parsing, targets and did-you-mean
//! (PLAN.md §6.3).

use pullpen_core::{MergeMethod, Pr, PrId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    Merge,
    Automerge,
    Approve,
    Comment,
    Checkout,
    Open,
    Diff,
    Close,
    Refresh,
    Section,
    Update,
    Dismiss,
    Messages,
    Clear,
    Quit,
    QuitAll,
}

pub struct CmdInfo {
    pub name: &'static str,
    pub cmd: Cmd,
    pub what: &'static str,
}

const fn c(name: &'static str, cmd: Cmd, what: &'static str) -> CmdInfo {
    CmdInfo { name, cmd, what }
}

/// Every command. Aliases are separate rows so they complete and parse, but
/// the wildmenu hides them (`shown`).
pub const COMMANDS: &[CmdInfo] = &[
    c("approve", Cmd::Approve, "approve the PR"),
    c("automerge", Cmd::Automerge, "arm or disarm auto-merge"),
    c("checkout", Cmd::Checkout, "check out the head branch"),
    c("clear", Cmd::Clear, "drop merged rows"),
    c("close", Cmd::Close, "close without merging"),
    c("comment", Cmd::Comment, "post a comment"),
    c("diff", Cmd::Diff, "open the diff"),
    c("merge", Cmd::Merge, "merge now"),
    c("messages", Cmd::Messages, "recent results and errors"),
    c("open", Cmd::Open, "open in the browser"),
    c("q", Cmd::Quit, "back, or quit from the list"),
    c("qa", Cmd::QuitAll, "quit from anywhere"),
    c("refresh", Cmd::Refresh, "refetch the section, or all"),
    c("section", Cmd::Section, "switch section"),
    c("update", Cmd::Update, "bring the branch up to date with its base"),
    c("dismiss", Cmd::Dismiss, "resolve a failure after a merge by hand"),
];

pub const ALIASES: &[(&str, Cmd)] = &[("am", Cmd::Automerge)];

pub fn lookup(name: &str) -> Option<Cmd> {
    COMMANDS
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.cmd)
        .or_else(|| ALIASES.iter().find(|(a, _)| *a == name).map(|(_, c)| *c))
}

/// The nearest command to a typo, if any is close enough to be worth
/// suggesting (Jaro-Winkler ≥ 0.8, PLAN.md §6.3).
pub fn did_you_mean(name: &str) -> Option<&'static str> {
    COMMANDS
        .iter()
        .map(|c| (strsim::jaro_winkler(name, c.name), c.name))
        .filter(|(score, _)| *score >= 0.8)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, n)| n)
}

/// A PR reference as typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetRef {
    /// `#812` or `!812`, optionally `repo#812`.
    Number { repo: Option<String>, number: u64 },
    /// A bare integer: the row as displayed, from 1.
    Row(usize),
}

pub fn parse_target(tok: &str) -> Option<TargetRef> {
    if let Ok(i) = tok.parse::<usize>() {
        return Some(TargetRef::Row(i));
    }
    let at = tok.find(['#', '!'])?;
    let number = tok[at + 1..].parse().ok()?;
    let repo = (at > 0).then(|| tok[..at].to_string());
    Some(TargetRef::Number { repo, number })
}

#[derive(Debug, PartialEq, Eq)]
pub enum Resolved {
    One(PrId),
    None,
    /// Two repos in the section share the number.
    Ambiguous(Vec<String>),
}

/// Resolves within the section's rows, as displayed.
pub fn resolve(t: &TargetRef, rows: &[Pr]) -> Resolved {
    match t {
        TargetRef::Row(i) => rows.get(i.wrapping_sub(1)).map_or(Resolved::None, |p| Resolved::One(p.id.clone())),
        TargetRef::Number { repo, number } => {
            let hits: Vec<&Pr> = rows
                .iter()
                .filter(|p| p.id.number == *number)
                .filter(|p| repo.as_ref().is_none_or(|r| p.id.repo == *r || p.id.repo_name() == r))
                .collect();
            match hits.as_slice() {
                [] => Resolved::None,
                [p] => Resolved::One(p.id.clone()),
                many => Resolved::Ambiguous(many.iter().map(|p| p.id.repo.clone()).collect()),
            }
        }
    }
}

pub fn parse_method(tok: &str) -> Option<MergeMethod> {
    match tok {
        "squash" | "s" => Some(MergeMethod::Squash),
        "merge" | "m" => Some(MergeMethod::Merge),
        "rebase" | "r" => Some(MergeMethod::Rebase),
        _ => None,
    }
}

/// A typed command line, split: the command, an optional target (only when
/// the first argument looks like one), and the rest.
#[derive(Debug, PartialEq, Eq)]
pub struct Parsed<'a> {
    pub name: &'a str,
    pub target: Option<TargetRef>,
    pub rest: &'a str,
}

pub fn parse(line: &str) -> Parsed<'_> {
    let line = line.trim_start();
    let (name, args) = line.split_once(' ').unwrap_or((line, ""));
    let args = args.trim_start();
    let (first, after) = args.split_once(' ').unwrap_or((args, ""));
    match parse_target(first) {
        Some(t) => Parsed { name, target: Some(t), rest: after.trim_start() },
        None => Parsed { name, target: None, rest: args },
    }
}

#[cfg(test)]
mod tests;
