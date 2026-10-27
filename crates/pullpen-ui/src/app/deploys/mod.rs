//! The Deploys view (ADR-023): one lane per repo and branch that merges
//! landed on, showing the newest merge there that ran a pipeline. A failing
//! lane needs attention while its branch still fails (ADR-019), until `D`.

use std::cmp::Reverse;
use std::collections::HashMap;

use pullpen_core::{PipelineState, Pr, PrId, View};

use super::merged::{CLEARED_MAX, merged_at};
use super::{App, Effect};
use crate::span::{Line, lr, markup};
use crate::views::armed::{Landed, landed, landed_why, lane_row};

impl App {
    /// A lane's group: a failure dismissed with `D` is done.
    pub(super) fn lane(&self, p: &Pr) -> Landed {
        match landed(p) {
            Landed::Attention if self.cleared.contains(&p.id) => Landed::Done,
            g => g,
        }
    }

    /// Reduces merged PRs to their lanes' heads, rings what changed since
    /// the last sync, and orders them deploying, failing, deployed.
    pub(super) fn lanes(&mut self, prs: &mut Vec<Pr>) {
        let mut heads: HashMap<(String, String, String), Pr> = HashMap::new();
        for p in prs.drain(..).filter(|p| p.post_merge.is_some()) {
            let key = (p.id.host.clone(), p.id.repo.clone(), p.base.clone());
            if heads.get(&key).is_none_or(|h| merged_at(&p) > merged_at(h)) {
                heads.insert(key, p);
            }
        }
        prs.extend(heads.into_values());
        for p in prs.iter() {
            let (g, n, base) = (self.lane(p), p.id.number, &p.base);
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
        let order = |p: &Pr| match self.lane(p) {
            Landed::Running => 0,
            Landed::Attention => 1,
            Landed::Done => 2,
        };
        prs.sort_by_key(|p| (order(p), Reverse(merged_at(p)), p.id.repo.clone()));
    }

    /// The lanes that need you, for the status line and the title.
    pub(super) fn failing(&self, prs: &[Pr]) -> usize {
        prs.iter().filter(|p| self.lane(p) == Landed::Attention).count()
    }

    /// Branches still failing after a merge, for the status line.
    pub(super) fn broken(&self) -> usize {
        let deploys = self.sections.iter().filter(|s| s.section.view == View::Deploys);
        deploys.map(|s| self.failing(&s.prs)).max().unwrap_or(0)
    }

    /// The terminal tab's title: the failing count rides along, so a broken
    /// branch shows from another tab.
    pub fn title(&self) -> String {
        match self.broken() {
            0 => "pullpen".into(),
            n => format!("pullpen ✗{n}"),
        }
    }

    /// `D`: a failing lane is resolved by hand, and stays resolved until a
    /// newer merge lands on its branch.
    pub(super) fn dismiss_here(&mut self, ids: &[PrId]) -> Vec<Effect> {
        if !self.view_is(self.active, View::Deploys) {
            self.refused("D dismisses a failing branch in Deploys");
            return vec![];
        }
        self.cleared.extend(ids.iter().cloned());
        // Dismissing is not the branch passing: nothing rings for it.
        for id in ids {
            self.landings.insert(id.clone(), Landed::Done);
        }
        let over = self.cleared.len().saturating_sub(CLEARED_MAX);
        self.cleared.drain(..over);
        for i in 0..self.sections.len() {
            if self.view_is(i, View::Deploys) {
                let mut prs = std::mem::take(&mut self.sections[i].prs);
                self.lanes(&mut prs);
                self.sections[i].total = prs.len();
                self.sections[i].prs = prs;
            }
        }
        vec![Effect::SaveCleared]
    }

    fn lane_why(&self, p: &Pr) -> String {
        match (&p.post_merge, landed(p) == Landed::Attention && self.cleared.contains(&p.id)) {
            (Some(pl), true) => format!("{{d|dismissed  {}}}", crate::views::detail::esc(&pl.job)),
            _ => landed_why(p),
        }
    }

    /// `✗ 1 failing   ·   2 deploying   ·   2 deployed`, what needs you first.
    fn deploys_summary(&self, w: usize) -> Line {
        let rows = self.section().visible();
        let count = |g| rows.iter().filter(|p| self.lane(p) == g).count();
        let parts: Vec<(&str, String)> = [
            (Landed::Attention, "{r|✗}", "failing"),
            (Landed::Running, "{y|○}", "deploying"),
            (Landed::Done, "{g|✓}", "deployed"),
        ]
        .into_iter()
        .filter(|(g, ..)| count(*g) > 0)
        .map(|(g, glyph, word)| (glyph, format!("{} {word}", count(g))))
        .collect();
        let left = match parts.split_first() {
            None => vec![],
            Some(((glyph, first), rest)) => {
                let rest: Vec<&str> = rest.iter().map(|(_, t)| t.as_str()).collect();
                let tail =
                    if rest.is_empty() { String::new() } else { format!("{{d|   ·   {}}}", rest.join("   ·   ")) };
                markup(&format!("{glyph} {{f|{first}}}{tail}"))
            }
        };
        lr(&left, &markup("{d|Enter pipeline   D dismiss   o open}"), w)
    }

    /// The Deploys view's lines, or `None` when the active section is not it.
    pub(super) fn deploys_lines(&self, w: usize, h: usize) -> Option<Vec<Line>> {
        if !self.view_is(self.active, View::Deploys) || self.detail.is_some() {
            return None;
        }
        let ctx = self.ctx();
        let sec = self.section();
        let rows = sec.visible();
        let groups = [(Landed::Running, "deploying"), (Landed::Attention, "failing"), (Landed::Done, "deployed")]
            .into_iter()
            .map(|(g, title)| {
                let members = rows
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| self.lane(p) == g)
                    .map(|(i, p)| (i, lane_row(p, w - 1, i == sec.sel, &self.lane_why(p), merged_at(p), &ctx)))
                    .collect();
                (title.to_string(), members)
            })
            .collect();
        Some(self.grouped(w, h, &sec.section.query, groups, self.deploys_summary(w)))
    }
}

#[cfg(test)]
mod tests;
