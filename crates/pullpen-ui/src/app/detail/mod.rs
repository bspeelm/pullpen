//! The detail view's keys and data (design §04).

use std::time::Instant;

use pullpen_core::diff::FileDiff;
use pullpen_core::{Action, PrDetail, PrId};

use super::act::Origin;
use super::{App, Effect};
use crate::keymap::Key;
use crate::views::detail::{Detail, Pane, body_rows, diff_rows, hunk_rows, overview};

impl App {
    pub fn open_detail(&mut self, zoom: bool) {
        if let Some(id) = self.section().selected().map(|p| p.id.clone()) {
            let mut d = Detail::new(id);
            d.zoom = zoom;
            if zoom {
                d.focus = Pane::Diff;
            }
            self.detail = Some(d);
        }
    }

    /// Fetches the view needs and has not asked for: the selected PR's
    /// detail for the preview, and the open PR's detail and diff.
    pub fn wants(&mut self) -> Vec<Effect> {
        let mut out = self.notifications();
        let sel = self.section().selected().map(|p| p.id.clone());
        let open = self.detail.as_ref().map(|d| d.id.clone());
        for id in sel.iter().chain(open.iter()) {
            if !self.details.contains_key(id) && self.requested.insert((id.clone(), false)) {
                out.push(Effect::FetchDetail(id.clone()));
            }
        }
        if let Some(id) = open
            && !self.diffs.contains_key(&id)
            && self.requested.insert((id.clone(), true))
        {
            out.push(Effect::FetchDiff(id));
        }
        out
    }

    pub fn detail_loaded(&mut self, id: PrId, d: PrDetail) {
        self.details.insert(id, d);
    }

    pub fn diff_loaded(&mut self, id: PrId, files: Vec<FileDiff>) {
        self.diffs.insert(id, files);
    }

    /// A fetch failed. It stays asked-for until the next sync, which forgets
    /// it, so a failing PR is retried at the sync interval rather than on
    /// every frame.
    pub fn fetch_failed(&mut self, id: &PrId, diff: bool) {
        self.failed.insert((id.clone(), diff));
    }

    fn pane_len(&self, d: &Detail) -> usize {
        match d.focus {
            Pane::Overview => {
                let empty = PrDetail::default();
                let pr = self.find(&d.id);
                pr.map_or(0, |_| overview(self.details.get(&d.id).unwrap_or(&empty), d.expanded, &self.ctx()).len())
            }
            Pane::Diff => self.diffs.get(&d.id).and_then(|f| f.get(d.file)).map_or(0, |f| diff_rows(f).len()),
        }
    }

    /// Keys while the detail view is open; `None` means the key is not the
    /// detail view's and falls through to the list's handling.
    pub(super) fn detail_key(&mut self, key: Key, now: Instant) -> Option<Vec<Effect>> {
        let mut d = self.detail.clone()?;
        if d.picker.is_some() {
            self.picker_key(&mut d, key);
            self.detail = Some(d);
            return Some(vec![]);
        }
        if let Some((first, _)) = self.pending.take() {
            self.detail_sequence(&mut d, first, key);
            self.detail = Some(d);
            return Some(vec![]);
        }
        let len = self.pane_len(&d);
        let page = (body_rows(self.height) / 2).max(1);
        let p = d.focus as usize;
        let max = len.saturating_sub(1);
        let id = d.id.clone();
        let ids = vec![id.clone()];
        let effects = match key {
            Key::Char('q') | Key::Esc => {
                self.detail = None;
                return Some(vec![]);
            }
            Key::Char('h') => {
                d.focus = Pane::Overview;
                vec![]
            }
            Key::Char('l') => {
                d.focus = Pane::Diff;
                vec![]
            }
            Key::Char('j') | Key::Down => {
                d.scroll[p] = (d.scroll[p] + 1).min(max);
                vec![]
            }
            Key::Char('k') | Key::Up => {
                d.scroll[p] = d.scroll[p].saturating_sub(1);
                vec![]
            }
            Key::Ctrl('d') => {
                d.scroll[p] = (d.scroll[p] + page).min(max);
                vec![]
            }
            Key::Ctrl('u') => {
                d.scroll[p] = d.scroll[p].saturating_sub(page);
                vec![]
            }
            Key::Char('G') => {
                d.scroll[p] = max;
                vec![]
            }
            Key::Char(c @ ('g' | ']' | '[' | 'z')) => {
                self.pending = Some((c, now));
                vec![]
            }
            Key::Char('n') | Key::Char('b') => {
                let files = self.diffs.get(&d.id).map_or(0, Vec::len);
                let to = if key == Key::Char('n') {
                    (d.file + 1).min(files.saturating_sub(1))
                } else {
                    d.file.saturating_sub(1)
                };
                if to != d.file {
                    d.file = to;
                    d.scroll[1] = 0;
                }
                d.focus = Pane::Diff;
                vec![]
            }
            Key::Char('F') => {
                if self.diffs.get(&d.id).is_some_and(|f| !f.is_empty()) {
                    d.picker = Some((String::new(), d.file));
                }
                vec![]
            }
            Key::Char('d') => {
                d.zoom = !d.zoom;
                if d.zoom {
                    d.focus = Pane::Diff;
                }
                vec![]
            }
            Key::Char('M') => {
                self.request_merge(ids, None, Origin::Key, None);
                vec![]
            }
            Key::Char('A') => self.request_arm(ids, None, Origin::Key, None),
            Key::Char('a') => self.request_simple(ids, Action::Approve, Origin::Key, None),
            Key::Char('x') => self.request_simple(ids, Action::Close, Origin::Key, None),
            Key::Char('u') => self.request_simple(ids, Action::Update, Origin::Key, None),
            Key::Char('c') => vec![Effect::Edit { targets: ids }],
            Key::Char('o') => self.urls(&ids).into_iter().map(Effect::Open).collect(),
            _ => {
                self.detail = Some(d);
                return None;
            }
        };
        self.detail = Some(d);
        Some(effects)
    }

    /// Keys while the file list is open: typing filters it, the arrows move,
    /// Enter opens the highlighted file, Esc closes.
    fn picker_key(&mut self, d: &mut Detail, key: Key) {
        let files = self.diffs.get(&d.id).cloned().unwrap_or_default();
        let Some((filter, sel)) = d.picker.as_mut() else { return };
        let shown = crate::views::detail::matching(&files, filter);
        match key {
            Key::Esc => d.picker = None,
            Key::Enter => {
                if let Some(&i) = shown.get(*sel) {
                    d.file = i;
                    d.scroll[1] = 0;
                    d.focus = Pane::Diff;
                }
                d.picker = None;
            }
            Key::Up | Key::Ctrl('p') => *sel = sel.saturating_sub(1),
            Key::Down | Key::Ctrl('n') => *sel = (*sel + 1).min(shown.len().saturating_sub(1)),
            Key::Backspace => {
                filter.pop();
                *sel = 0;
            }
            Key::Char(c) => {
                filter.push(c);
                *sel = 0;
            }
            _ => {}
        }
    }

    fn detail_sequence(&mut self, d: &mut Detail, first: char, key: Key) {
        let files = self.diffs.get(&d.id).map_or(0, Vec::len);
        let file = self.diffs.get(&d.id).and_then(|f| f.get(d.file)).cloned();
        match (first, key) {
            ('g', Key::Char('g')) => d.scroll[d.focus as usize] = 0,
            (']', Key::Char('f')) if d.file + 1 < files => {
                d.file += 1;
                d.scroll[1] = 0;
            }
            ('[', Key::Char('f')) if d.file > 0 => {
                d.file -= 1;
                d.scroll[1] = 0;
            }
            (']' | '[', Key::Char('c')) => {
                let rows = file.as_ref().map(hunk_rows).unwrap_or_default();
                let at = d.scroll[1];
                let next =
                    if first == ']' { rows.iter().find(|r| **r > at) } else { rows.iter().rev().find(|r| **r < at) };
                match next {
                    Some(r) => d.scroll[1] = *r,
                    // Past the last hunk, on to the next file's first.
                    None if first == ']' && d.file + 1 < files => {
                        d.file += 1;
                        d.scroll[1] = 1;
                    }
                    None => {}
                }
                d.focus = Pane::Diff;
            }
            ('z', Key::Char('o')) => d.expanded = true,
            ('z', Key::Char('c')) => d.expanded = false,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
