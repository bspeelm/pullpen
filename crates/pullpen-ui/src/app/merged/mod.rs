//! The Merged view (ADR-017, ADR-023): what merged this week, newest first,
//! each with what its merge commit's pipeline did. Acting on a failure is
//! the Deploys view's job.

use std::cmp::Reverse;

use pullpen_core::time::Timestamp;
use pullpen_core::{Pr, PrId, PrState, View};

use super::{App, Effect};
use crate::span::{Line, lr, markup};
use crate::views::armed::{Landed, history_why, landed, row};

/// PRs cleared or dismissed, remembered; older ones are long out of any
/// merged window.
pub const CLEARED_MAX: usize = 500;

pub(super) fn merged_at(p: &Pr) -> Timestamp {
    if let PrState::Merged { at, .. } = p.state { at } else { p.updated_at }
}

impl App {
    /// Drops what `:clear` dropped, newest merge first. A failure stays:
    /// `:clear` never hid one, and `D` in Deploys is for a branch, not the record.
    pub(super) fn history(&self, prs: &mut Vec<Pr>) {
        prs.retain(|p| !self.cleared.contains(&p.id) || landed(p) == Landed::Attention);
        prs.sort_by_key(|p| Reverse(merged_at(p)));
    }

    /// Takes the PRs cleared or dismissed in earlier sessions.
    pub fn set_cleared(&mut self, ids: Vec<PrId>) {
        self.cleared = ids;
        self.arrange();
    }

    /// `:clear` in the Merged view: every finished row that is not failing
    /// goes, and stays gone.
    pub(super) fn clear_landed(&mut self) -> Vec<Effect> {
        for sec in self.sections.iter_mut().filter(|s| s.section.view == View::Merged) {
            let (out, keep): (Vec<Pr>, Vec<Pr>) =
                std::mem::take(&mut sec.prs).into_iter().partition(|p| landed(p) == Landed::Done);
            self.cleared.extend(out.into_iter().map(|p| p.id));
            sec.total = keep.len();
            sec.prs = keep;
            sec.sel = sec.sel.min(sec.prs.len().saturating_sub(1));
        }
        let over = self.cleared.len().saturating_sub(CLEARED_MAX);
        self.cleared.drain(..over);
        vec![Effect::SaveCleared]
    }

    /// The Merged view's lines, or `None` when the active section is not it.
    pub(super) fn merged_lines(&self, w: usize, h: usize) -> Option<Vec<Line>> {
        if !self.view_is(self.active, View::Merged) || self.detail.is_some() {
            return None;
        }
        let ctx = self.ctx();
        let sec = self.section();
        let rows = sec.visible();
        let members = rows
            .iter()
            .enumerate()
            .map(|(i, p)| (i, row(p, w - 1, i == sec.sel, &history_why(p, ctx.now), merged_at(p), true, &ctx)));
        let groups = vec![("last 7 days".to_string(), members.collect())];
        let summary =
            lr(&markup(&format!("{{d|{} merged}}", rows.len())), &markup("{d|w pipeline   :clear drop finished}"), w);
        Some(self.grouped(w, h, &sec.section.query, groups, summary))
    }
}

#[cfg(test)]
mod tests;
