//! What the preview and the detail view show beyond the list row. Fetched per
//! PR on demand, never for a whole section.

use crate::time::Timestamp;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrDetail {
    pub description: String,
    pub checks: Vec<CheckLine>,
    /// The timeline, newest first: reviews, arming, pushes, requests.
    pub activity: Vec<Activity>,
    /// Reviews and comments with their bodies, as the detail view lists them.
    pub conversation: Vec<Activity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckState {
    Passed,
    Running,
    Failed,
    Queued,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckLine {
    pub name: String,
    pub state: CheckState,
    pub required: bool,
    /// How long it took, or has been running, in the forge's own words.
    pub duration: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActivityKind {
    Approved,
    ChangesRequested,
    ArmedAutoMerge,
    DisarmedAutoMerge,
    Pushed { commits: u32 },
    ReviewRequested { by: Option<String> },
    Commented,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activity {
    pub who: String,
    pub is_agent: bool,
    pub kind: ActivityKind,
    pub at: Timestamp,
    /// A review or comment body, or the message of a single pushed commit.
    pub body: String,
}
