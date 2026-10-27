//! What the preview and the detail view show beyond the list row. Fetched per
//! PR on demand, never for a whole section.

use crate::time::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrDetail {
    pub description: String,
    pub checks: Vec<CheckLine>,
    /// Wall time of the latest check run, first start to last finish.
    pub checks_took: String,
    /// The timeline, newest first: reviews, arming, pushes, requests.
    pub activity: Vec<Activity>,
    /// Reviews and comments with their bodies, as the detail view lists them:
    /// the most recent few, out of `conversation_total`.
    pub conversation: Vec<Activity>,
    pub conversation_total: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckState {
    Passed,
    Running,
    Failed,
    Queued,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckLine {
    pub name: String,
    pub state: CheckState,
    pub required: bool,
    /// How long it took, or has been running, in the forge's own words.
    pub duration: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityKind {
    Approved,
    ChangesRequested,
    ArmedAutoMerge,
    DisarmedAutoMerge,
    Pushed { commits: u32 },
    ReviewRequested { by: Option<String> },
    Commented,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub who: String,
    pub is_agent: bool,
    pub kind: ActivityKind,
    pub at: Timestamp,
    /// A review or comment body, or the message of a single pushed commit.
    pub body: String,
}

/// Which of a PR's pipelines a run is: its head's, or its merge commit's on
/// the base branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Which {
    BeforeMerge,
    AfterMerge,
}

/// One pipeline run of a PR, as the pipeline screen shows it (ADR-020).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub which: Which,
    pub sha: String,
    /// What started it, in the forge's words: `push`, `merge request`.
    pub event: String,
    pub state: CheckState,
    pub started: Option<Timestamp>,
    pub finished: Option<Timestamp>,
    pub url: String,
    pub jobs: Vec<JobRun>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRun {
    /// What the forge's log endpoint takes.
    pub id: String,
    pub name: String,
    /// GitLab's stage, or GitHub's workflow.
    pub group: String,
    pub state: CheckState,
    pub started: Option<Timestamp>,
    pub finished: Option<Timestamp>,
    /// GitHub's steps. GitLab has none; its log's sections stand in.
    pub steps: Vec<Step>,
    pub url: String,
    /// False for a commit status from outside the forge's CI, which has
    /// only a link.
    pub log: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    pub state: CheckState,
    pub started: Option<Timestamp>,
    pub finished: Option<Timestamp>,
}

impl Run {
    pub fn done(&self) -> bool {
        matches!(self.state, CheckState::Passed | CheckState::Failed)
    }
}

/// Running while any job has yet to finish, then failed if any failed.
pub fn overall(jobs: &[JobRun]) -> CheckState {
    let any = |st: CheckState| jobs.iter().any(|j| j.state == st);
    match () {
        _ if any(CheckState::Running) || any(CheckState::Queued) => CheckState::Running,
        _ if any(CheckState::Failed) => CheckState::Failed,
        _ => CheckState::Passed,
    }
}
