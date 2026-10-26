//! The armed lifecycle (design §06, PLAN.md §6.5): armed → waiting →
//! merged, or → needs attention; the two transitions out of waiting are
//! worth interrupting someone for.

use pullpen_core::time::Timestamp;
use pullpen_core::{Pr, PrId, View};

use super::{App, Effect, Input};
use crate::span::{Line, markup};
use crate::views::armed::{Group, clock, group, heading, row, why};
use crate::views::chrome::tabs;

/// What pullpen last saw of an armed PR, and since when.
#[derive(Clone, Debug)]
pub struct Watch {
    pub group: Group,
    pub since: Timestamp,
    /// The forge turned auto-merge off without pullpen asking.
    pub disarmed: bool,
}

impl App {
    pub(super) fn view_is(&self, i: usize, v: View) -> bool {
        self.sections.get(i).is_some_and(|s| s.section.view == v)
    }

    fn group_of(&self, p: &Pr) -> Option<Group> {
        let disarmed = self.watches.get(&p.id).is_some_and(|w| w.disarmed);
        group(p, self.readiness(p), disarmed)
    }

    /// Updates the watches from a sync, queues notifications, and orders an
    /// armed section by group. Called by `synced` before the rows land.
    pub(super) fn track(&mut self, i: usize, prs: &mut [Pr]) {
        let now = self.now;
        for p in prs.iter() {
            let prev = self.watches.get(&p.id).cloned();
            let disarmed = prev.as_ref().is_some_and(|w| w.group == Group::Waiting && !p.armed() && !p.merged());
            let Some(g) = group(p, self.readiness(p), disarmed || prev.as_ref().is_some_and(|w| w.disarmed)) else {
                continue;
            };
            if prev.as_ref().is_some_and(|w| w.group == g) {
                continue;
            }
            if prev.as_ref().is_some_and(|w| w.group == Group::Waiting) {
                let n = p.id.number;
                self.notify.push(match g {
                    Group::Merged => format!("#{n} merged"),
                    _ if disarmed => format!("#{n} auto-merge was turned off"),
                    _ => format!("#{n} needs attention"),
                });
            }
            // A transition happened now; a PR seen for the first time went into
            // its state at some point pullpen missed, best dated by its last update.
            let since = match (g, &prev) {
                (Group::Waiting, _) => p.auto_merge.as_ref().map_or(now, |a| a.armed_at),
                (_, None) => p.updated_at,
                _ => now,
            };
            self.watches.insert(
                p.id.clone(),
                Watch { group: g, since, disarmed: disarmed || prev.is_some_and(|w| w.disarmed) },
            );
        }
        if self.view_is(i, View::Armed) {
            prs.sort_by_key(|p| self.group_of(p).unwrap_or(Group::Waiting));
        }
    }

    /// Groups the armed sections as loaded (from the cache, or fixtures),
    /// before any sync: first sight rings nothing.
    pub(super) fn arrange(&mut self) {
        for i in 0..self.sections.len() {
            if self.view_is(i, View::Armed) || self.view_is(i, View::Merged) {
                let mut prs = std::mem::take(&mut self.sections[i].prs);
                self.track(i, &mut prs);
                if self.view_is(i, View::Merged) {
                    self.land(&mut prs);
                    self.sections[i].total = prs.len();
                }
                self.sections[i].prs = prs;
            }
        }
        self.notify.clear();
    }

    /// Notifications waiting to be rung, as effects.
    pub(super) fn notifications(&mut self) -> Vec<Effect> {
        self.notify.drain(..).map(Effect::Notify).collect()
    }

    /// pullpen asked for it: no watch, so no notification for its outcome.
    pub(super) fn unwatch(&mut self, id: &PrId) {
        self.watches.remove(id);
    }

    /// When a row's clock starts: when it went bad, when it was armed, when
    /// it merged.
    fn when(&self, p: &Pr, g: Group) -> Timestamp {
        match (g, &p.state, self.watches.get(&p.id)) {
            (Group::Merged, pullpen_core::PrState::Merged { at, .. }, _) => *at,
            (Group::Waiting, _, _) => p.auto_merge.as_ref().map_or(self.now, |a| a.armed_at),
            (_, _, Some(w)) => w.since,
            _ => self.now,
        }
    }

    pub fn since_label(&self) -> String {
        self.last_visit.map_or("this session".into(), |t| format!("since {}", clock(t, self.tz_offset)))
    }

    /// `◆ 4 merged since 09:14 · 2 need attention · 5 still waiting`.
    pub fn armed_summary(&self) -> Line {
        let rows = self.section().visible();
        let count = |g| rows.iter().filter(|p| self.group_of(p) == Some(g)).count();
        let (m, a, w) = (count(Group::Merged), count(Group::Attention), count(Group::Waiting));
        let mut parts = Vec::new();
        if m > 0 {
            parts.push(format!("{m} merged {}", self.since_label()));
        }
        if a > 0 {
            parts.push(format!("{a} need{} attention", if a == 1 { "s" } else { "" }));
        }
        if w > 0 {
            parts.push(format!("{w} still waiting"));
        }
        let Some((first, rest)) = parts.split_first() else { return vec![] };
        let tail = if rest.is_empty() { String::new() } else { format!("{{d|   ·   {}}}", rest.join("   ·   ")) };
        markup(&format!("{{a|◆}} {{f|{first}}}{tail}"))
    }

    /// The armed view's lines, or `None` when the active section is not it.
    pub(super) fn armed_lines(&self, w: usize, h: usize) -> Option<Vec<Line>> {
        if !self.view_is(self.active, View::Armed) || self.detail.is_some() {
            return None;
        }
        let ctx = self.ctx();
        let sec = self.section();
        let rows = sec.visible();
        let all: Vec<Pr> = rows.iter().map(|p| (*p).clone()).collect();
        let groups = [Group::Attention, Group::Waiting, Group::Merged]
            .into_iter()
            .map(|g| {
                let title = match g {
                    Group::Attention => "needs attention".to_string(),
                    Group::Waiting => "waiting".into(),
                    Group::Merged => format!("merged {}", self.since_label()),
                };
                let members = rows
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| self.group_of(p) == Some(g))
                    .map(|(i, p)| {
                        let disarmed = self.watches.get(&p.id).is_some_and(|w| w.disarmed);
                        let why = why(p, g, disarmed, &all);
                        (i, row(p, w - 1, i == sec.sel, &why, self.when(p, g), false, &ctx))
                    })
                    .collect();
                (title, members)
            })
            .collect();
        let right = format!("{} + recent merges", sec.section.query);
        Some(self.grouped(w, h, &right, groups, self.armed_summary()))
    }

    /// A grouped view's frame: tabs, each non-empty group under its
    /// heading, scrolled to keep the cursor in view, then the status line
    /// and the summary while nothing else wants the last row.
    pub(super) fn grouped(
        &self,
        w: usize,
        h: usize,
        right: &str,
        groups: Vec<(String, Vec<(usize, Line)>)>,
        summary: Line,
    ) -> Vec<Line> {
        let sec = self.section();
        let tab: Vec<_> = self
            .sections
            .iter()
            .map(|x| crate::views::chrome::Tab {
                name: x.section.name.clone(),
                short: x.section.short.clone(),
                count: x.total,
            })
            .collect();
        let mut out = vec![tabs(&tab, self.active, w, self.narrow(w), right), vec![]];
        let (mut body, mut sel_line) = (Vec::new(), 0);
        for (title, members) in groups.into_iter().filter(|(_, m)| !m.is_empty()) {
            body.push(heading(&title, members.len()));
            for (i, line) in members {
                if i == sec.sel {
                    sel_line = body.len();
                }
                body.push(line);
            }
            body.push(vec![]);
        }
        let room = h.saturating_sub(4);
        let start = sel_line.saturating_sub(room.saturating_sub(1));
        out.extend(body.into_iter().skip(start).take(room));
        while out.len() < h.saturating_sub(2) {
            out.push(vec![]);
        }
        out.push(crate::views::chrome::status(w, &self.status(w)));
        let cmd = match (&self.input, &self.message) {
            (Input::Normal, None) if self.jobs.is_empty() => summary,
            _ => self.cmd_line(w),
        };
        out.push(cmd);
        out.into_iter().map(|l| crate::span::fit(&l, w, false)).collect()
    }

    /// Rows still armed: what the tab counts and the position counts in.
    pub(super) fn armed_count(&self, prs: &[Pr]) -> usize {
        prs.iter().filter(|p| !p.merged()).count()
    }

    pub fn clear_merged(&mut self) {
        for sec in self.sections.iter_mut().filter(|s| s.section.view == View::Armed) {
            let gone: Vec<PrId> = sec.prs.iter().filter(|p| p.merged()).map(|p| p.id.clone()).collect();
            sec.prs.retain(|p| !p.merged());
            sec.sel = sec.sel.min(sec.prs.len().saturating_sub(1));
            for id in gone {
                self.watches.remove(&id);
            }
        }
    }
}

#[cfg(test)]
mod tests;
