//! What the user asks for, and what comes back (PLAN.md §3).

use crate::pr::{MergeMethod, PrId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Merge { method: MergeMethod },
    Arm { method: MergeMethod },
    Disarm,
    Approve,
    Comment(String),
    Close,
    Checkout,
    Open,
    Refresh { all: bool },
}

impl Action {
    /// The command that names this action, for messages: `:approve #2290: …`.
    pub fn command(&self) -> &'static str {
        match self {
            Action::Merge { .. } => "merge",
            Action::Arm { .. } | Action::Disarm => "automerge",
            Action::Approve => "approve",
            Action::Comment(_) => "comment",
            Action::Close => "close",
            Action::Checkout => "checkout",
            Action::Open => "open",
            Action::Refresh { .. } => "refresh",
        }
    }
}

/// One action on one PR, as the forge layer receives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub id: u64,
    pub target: PrId,
    pub action: Action,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Done,
    /// The forge refused. `reason` is its explanation, in its words.
    Failed {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobResult {
    pub job: Job,
    pub outcome: Outcome,
}
