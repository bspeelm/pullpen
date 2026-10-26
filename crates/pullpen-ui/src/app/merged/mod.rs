//! The Merged view (ADR-017): merged PRs grouped by what their merge
//! commit's pipeline did on the base branch. A row leaves a day after its
//! merge, except a failure, which stays until `:clear`: being away is when
//! a failed deploy most needs to still be on screen.

use std::cmp::Reverse;

use pullpen_core::time::Timestamp;
use pullpen_core::{PipelineState, Pr, PrId, PrState, View};

use super::{App, Effect};
use crate::span::{Line, markup};
use crate::views::armed::{Landed, landed, landed_why, row};

/// How long a row that did not fail stays after its merge.
const KEEP: i64 = 86_400;
/// Cleared PRs remembered; older ones are long out of any merged window.
pub const CLEARED_MAX: usize = 500;

fn merged_at(p: &Pr) -> Timestamp {
    if let PrState::Merged { at, .. } = p.state { at } else { p.updated_at }
}

impl App {
    /// Drops rows that have left the view, queues a notification for each
    /// pipeline seen running that has since finished, and orders the rest
    /// by group, newest merge first.
    pub(super) fn land(&mut self, prs: &mut Vec<Pr>) {
        let now = self.now.0;
        prs.retain(|p| !self.cleared.contains(&p.id) && (now - merged_at(p).0 < KEEP || landed(p) == Landed::Failed));
        for p in prs.iter() {
            let state = p.post_merge.as_ref().map(|x| x.state);
            let was = self.landings.insert(p.id.clone(), state);
            if was == Some(Some(PipelineState::Running)) {
                let n = p.id.number;
                match &p.post_merge {
                    Some(pl) if pl.state == PipelineState::Failed => {
                        self.notify.push(format!("#{n} failed on {} after merging {}", p.base, pl.job).trim().into())
                    }
                    Some(pl) if pl.state == PipelineState::Passed => {
                        self.notify.push(format!("#{n} passed on {} after merging", p.base))
                    }
                    _ => {}
                }
            }
        }
        prs.sort_by_key(|p| (landed(p), Reverse(merged_at(p))));
    }

    /// Takes the PRs `:clear` dropped in earlier sessions, and drops them.
    pub fn set_cleared(&mut self, ids: Vec<PrId>) {
        self.cleared = ids;
        for i in 0..self.sections.len() {
            if self.view_is(i, View::Merged) {
                let mut prs = std::mem::take(&mut self.sections[i].prs);
                self.land(&mut prs);
                self.sections[i].total = prs.len();
                self.sections[i].prs = prs;
            }
        }
        self.notify.clear();
    }

    /// `:clear` in the Merged view: every finished row goes, and stays gone.
    pub(super) fn clear_landed(&mut self) -> Vec<Effect> {
        for sec in self.sections.iter_mut().filter(|s| s.section.view == View::Merged) {
            let (done, running): (Vec<Pr>, Vec<Pr>) =
                std::mem::take(&mut sec.prs).into_iter().partition(|p| landed(p) != Landed::Running);
            self.cleared.extend(done.into_iter().map(|p| p.id));
            sec.total = running.len();
            sec.prs = running;
            sec.sel = sec.sel.min(sec.prs.len().saturating_sub(1));
        }
        let over = self.cleared.len().saturating_sub(CLEARED_MAX);
        self.cleared.drain(..over);
        vec![Effect::SaveCleared]
    }

    /// `✗ 1 failed   ·   2 running   ·   3 done`, led by the most urgent.
    fn merged_summary(&self) -> Line {
        let rows = self.section().visible();
        let count = |g| rows.iter().filter(|p| landed(p) == g).count();
        let parts: Vec<(&str, String)> = [
            (Landed::Failed, "{r|✗}", "failed"),
            (Landed::Running, "{y|○}", "running"),
            (Landed::Done, "{g|✓}", "done"),
        ]
        .into_iter()
        .filter(|(g, ..)| count(*g) > 0)
        .map(|(g, glyph, word)| (glyph, format!("{} {word}", count(g))))
        .collect();
        let Some(((glyph, first), rest)) = parts.split_first() else { return vec![] };
        let rest: Vec<&str> = rest.iter().map(|(_, t)| t.as_str()).collect();
        let tail = if rest.is_empty() { String::new() } else { format!("{{d|   ·   {}}}", rest.join("   ·   ")) };
        markup(&format!("{glyph} {{f|{first}}}{tail}"))
    }

    /// The Merged view's lines, or `None` when the active section is not it.
    pub(super) fn merged_lines(&self, w: usize, h: usize) -> Option<Vec<Line>> {
        if !self.view_is(self.active, View::Merged) || self.detail.is_some() {
            return None;
        }
        let ctx = self.ctx();
        let sec = self.section();
        let rows = sec.visible();
        let groups = [(Landed::Failed, "failed"), (Landed::Running, "running"), (Landed::Done, "done")]
            .into_iter()
            .map(|(g, title)| {
                let members = rows
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| landed(p) == g)
                    .map(|(i, p)| (i, row(p, w - 1, i == sec.sel, &landed_why(p), merged_at(p), true, &ctx)))
                    .collect();
                (title.to_string(), members)
            })
            .collect();
        Some(self.grouped(w, h, &sec.section.query, groups, self.merged_summary()))
    }
}

#[cfg(test)]
mod tests;
