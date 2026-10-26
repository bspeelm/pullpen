//! The merge verdict: READY, WAITING or BLOCKED, and on what (design §04).
//!
//! This decides; the UI phrases. Keeping the sentences out of here means the
//! preview's short form and the detail view's long form cannot disagree
//! about the facts.

use crate::pr::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wait {
    Ci {
        passed: u32,
        total: u32,
    },
    Reviews {
        have: u32,
        need: u32,
        pending: Vec<String>,
    },
    Train {
        position: u32,
        length: u32,
    },
    Behind {
        base: String,
    },
    Mergeability,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Draft,
    Conflict { base: String },
    Failing { check: String },
    ChangesRequested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Ready,
    /// Only time or other people are needed; never empty.
    Waiting(Vec<Wait>),
    /// The first blocker, which is the one the sentence names.
    Blocked(Block),
}

pub fn verdict(pr: &Pr, caps: &Capabilities) -> Verdict {
    if pr.draft {
        return Verdict::Blocked(Block::Draft);
    }
    if pr.mergeable == Mergeable::Conflict {
        return Verdict::Blocked(Block::Conflict {
            base: pr.base.clone(),
        });
    }
    if let Some(c) = pr.checks.failed.first() {
        return Verdict::Blocked(Block::Failing {
            check: c.name.clone(),
        });
    }
    if pr.reviews.changes_requested {
        return Verdict::Blocked(Block::ChangesRequested);
    }
    let mut waits = Vec::new();
    if !pr.checks.all_passed() {
        waits.push(Wait::Ci {
            passed: pr.checks.required_passed,
            total: pr.checks.required_total,
        });
    }
    if pr.reviews.short() {
        waits.push(Wait::Reviews {
            have: pr.reviews.have,
            need: pr.reviews.need,
            pending: pr.reviews.pending.clone(),
        });
    }
    if pr.mergeable == Mergeable::Behind && caps.requires_up_to_date {
        waits.push(Wait::Behind {
            base: pr.base.clone(),
        });
    }
    if pr.mergeable == Mergeable::Unknown {
        waits.push(Wait::Mergeability);
    }
    if let Some(t) = pr.merge_train {
        waits.push(Wait::Train {
            position: t.position,
            length: t.length,
        });
    }
    if waits.is_empty() {
        Verdict::Ready
    } else {
        Verdict::Waiting(waits)
    }
}
