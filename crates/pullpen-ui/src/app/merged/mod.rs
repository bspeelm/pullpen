//! The Merged view (ADR-017): merged PRs grouped by what their merge
//! commit's pipeline did on the base branch. A failure needs attention while
//! the branch is still failing, and stays until `D` or `:clear`: being away
//! is when a broken deploy most needs to still be on screen. Everything
//! else, a failure fixed since included (ADR-019), leaves a day after merging.

use std::cmp::Reverse;
use std::collections::HashSet;

use pullpen_core::time::Timestamp;
use pullpen_core::{PipelineState, Pr, PrId, PrState, View};

use super::{App, Effect};
use crate::span::{Line, markup};
use crate::views::armed::{Landed, landed, landed_why, row};

/// How long a row that needs nothing stays after its merge.
const KEEP: i64 = 86_400;
/// Cleared PRs remembered; older ones are long out of any merged window.
pub const CLEARED_MAX: usize = 500;

fn merged_at(p: &Pr) -> Timestamp {
    if let PrState::Merged { at, .. } = p.state { at } else { p.updated_at }
}

/// What the Merged tab counts: what needs you, not everything merged.
pub(super) fn attention(prs: &[Pr]) -> usize {
    prs.iter().filter(|p| landed(p) == Landed::Attention).count()
}

impl App {
    /// Drops rows that have left the view, queues a notification for each
    /// row that broke its branch or saw it fixed since the last sync, and
    /// orders the rest by group, newest merge first.
    pub(super) fn land(&mut self, prs: &mut Vec<Pr>) {
        let now = self.now.0;
        prs.retain(|p| {
            !self.cleared.contains(&p.id) && (now - merged_at(p).0 < KEEP || landed(p) == Landed::Attention)
        });
        for p in prs.iter() {
            let (g, n, base) = (landed(p), p.id.number, &p.base);
            let job = p.post_merge.as_ref().map_or("", |x| x.job.as_str());
            let passed = p.post_merge.as_ref().is_some_and(|x| x.state == PipelineState::Passed);
            match (self.landings.insert(p.id.clone(), g), g) {
                (Some(was), Landed::Attention) if was != Landed::Attention => {
                    self.notify.push(format!("{base} is failing after #{n}: {job}").trim_end_matches(": ").into())
                }
                (Some(Landed::Attention), Landed::Done) => self.notify.push(format!("{base} passes again after #{n}")),
                (Some(Landed::Running), Landed::Done) if passed => {
                    self.notify.push(format!("#{n} passed on {base} after merging"))
                }
                _ => {}
            }
        }
        prs.sort_by_key(|p| (landed(p), Reverse(merged_at(p))));
    }

    /// Base branches still failing after a merge, for the status line.
    pub(super) fn broken(&self) -> usize {
        let merged = self.sections.iter().filter(|s| s.section.view == View::Merged);
        merged
            .flat_map(|s| &s.prs)
            .filter(|p| landed(p) == Landed::Attention)
            .map(|p| &p.id)
            .collect::<HashSet<_>>()
            .len()
    }

    /// The terminal tab's title: the attention count rides along, so a broken
    /// branch shows from another tab.
    pub fn title(&self) -> String {
        match self.broken() {
            0 => "pullpen".into(),
            n => format!("pullpen ✗{n}"),
        }
    }

    /// `D`: the rows are resolved by hand, and stay gone. Only merged rows
    /// can be; anywhere else it says where it works.
    pub(super) fn dismiss_here(&mut self, ids: &[PrId]) -> Vec<Effect> {
        if !self.view_is(self.active, View::Merged) {
            self.refused("D dismisses a failure in the Merged section");
            return vec![];
        }
        self.forget(|p| ids.contains(&p.id))
    }

    /// Takes the PRs `:clear` dropped in earlier sessions, and drops them.
    pub fn set_cleared(&mut self, ids: Vec<PrId>) {
        self.cleared = ids;
        for i in 0..self.sections.len() {
            if self.view_is(i, View::Merged) {
                let mut prs = std::mem::take(&mut self.sections[i].prs);
                self.land(&mut prs);
                self.sections[i].total = attention(&prs);
                self.sections[i].prs = prs;
            }
        }
        self.notify.clear();
    }

    /// `:clear` in the Merged view: every finished row goes, and stays gone.
    pub(super) fn clear_landed(&mut self) -> Vec<Effect> {
        self.forget(|p| landed(p) != Landed::Running)
    }

    /// Drops the merged rows `gone` picks, remembering them so a sync does
    /// not bring them back.
    fn forget(&mut self, gone: impl Fn(&Pr) -> bool) -> Vec<Effect> {
        for sec in self.sections.iter_mut().filter(|s| s.section.view == View::Merged) {
            let (out, keep): (Vec<Pr>, Vec<Pr>) = std::mem::take(&mut sec.prs).into_iter().partition(&gone);
            self.cleared.extend(out.into_iter().map(|p| p.id));
            sec.total = attention(&keep);
            sec.prs = keep;
            sec.sel = sec.sel.min(sec.prs.len().saturating_sub(1));
        }
        let over = self.cleared.len().saturating_sub(CLEARED_MAX);
        self.cleared.drain(..over);
        vec![Effect::SaveCleared]
    }

    /// `✗ 1 needs attention   ·   2 running   ·   3 done`, most urgent first.
    fn merged_summary(&self) -> Line {
        let rows = self.section().visible();
        let count = |g| rows.iter().filter(|p| landed(p) == g).count();
        let parts: Vec<(&str, String)> = [
            (Landed::Attention, "{r|✗}", "needs attention"),
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
        let groups = [(Landed::Attention, "needs attention"), (Landed::Running, "running"), (Landed::Done, "done")]
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
