//! A pull request as pullpen sees it, whichever forge it came from.

use crate::time::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Forge {
    GitHub,
    GitLab,
}

impl Forge {
    /// The two-letter tag the list shows.
    pub fn tag(self) -> &'static str {
        match self {
            Forge::GitHub => "gh",
            Forge::GitLab => "gl",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrId {
    pub forge: Forge,
    pub host: String,
    /// `owner/name`, or a GitLab project path, which may have more segments.
    pub repo: String,
    pub number: u64,
}

impl PrId {
    /// The repo's last path segment: what the list falls back to when the
    /// full path no longer fits (design §11).
    pub fn repo_name(&self) -> &str {
        self.repo.rsplit('/').next().unwrap_or(&self.repo)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Author {
    pub login: String,
    pub is_agent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckRun {
    pub name: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checks {
    pub required_total: u32,
    pub required_passed: u32,
    pub running: u32,
    pub failed: Vec<CheckRun>,
}

impl Checks {
    pub fn failing(&self) -> bool {
        !self.failed.is_empty()
    }
    pub fn all_passed(&self) -> bool {
        !self.failing() && self.running == 0 && self.required_passed >= self.required_total
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reviews {
    pub have: u32,
    pub need: u32,
    pub changes_requested: bool,
    pub pending: Vec<String>,
}

impl Reviews {
    pub fn short(&self) -> bool {
        self.have < self.need
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mergeable {
    Clean,
    Conflict,
    Behind,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MergeMethod {
    Squash,
    Merge,
    Rebase,
}

impl MergeMethod {
    pub fn word(self) -> &'static str {
        match self {
            MergeMethod::Squash => "squash",
            MergeMethod::Merge => "merge",
            MergeMethod::Rebase => "rebase",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoMerge {
    pub method: MergeMethod,
    pub armed_by: String,
    pub armed_at: Timestamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeTrainPos {
    pub position: u32,
    pub length: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrState {
    Open,
    Merged { at: Timestamp, method: MergeMethod },
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pr {
    pub id: PrId,
    pub title: String,
    pub author: Author,
    pub head: String,
    pub base: String,
    pub additions: u32,
    pub deletions: u32,
    pub files: u32,
    pub draft: bool,
    pub checks: Checks,
    pub reviews: Reviews,
    pub mergeable: Mergeable,
    pub auto_merge: Option<AutoMerge>,
    pub merge_train: Option<MergeTrainPos>,
    pub state: PrState,
    pub updated_at: Timestamp,
    pub created_at: Timestamp,
    pub url: String,
    pub repo_default_method: MergeMethod,
    pub repo_allowed_methods: Vec<MergeMethod>,
}

impl Pr {
    pub fn merged(&self) -> bool {
        matches!(self.state, PrState::Merged { .. })
    }
    pub fn armed(&self) -> bool {
        self.auto_merge.is_some()
    }
}

/// What a repo demands before a merge, as far as pullpen needs to know.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub allowed_methods: Vec<MergeMethod>,
    pub merge_trains: bool,
    pub requires_up_to_date: bool,
}

/// The one-cell answer to "safe to merge?". Variants are in glyph precedence
/// order: the first that applies wins (design §01, `stateG`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Readiness {
    Merged,
    Draft,
    Conflict,
    Failing,
    Waiting,
    Ready,
}

pub fn readiness(pr: &Pr, caps: &Capabilities) -> Readiness {
    if pr.merged() {
        Readiness::Merged
    } else if pr.draft {
        Readiness::Draft
    } else if pr.mergeable == Mergeable::Conflict {
        Readiness::Conflict
    } else if pr.checks.failing() || pr.reviews.changes_requested {
        Readiness::Failing
    } else if !pr.checks.all_passed()
        || pr.reviews.short()
        || pr.mergeable == Mergeable::Unknown
        || (pr.mergeable == Mergeable::Behind && caps.requires_up_to_date)
    {
        Readiness::Waiting
    } else {
        Readiness::Ready
    }
}

#[cfg(test)]
mod tests;
