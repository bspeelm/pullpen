//! Keys to state changes and effects (PLAN.md §6.2).

use std::time::{Duration, Instant};

use pullpen_core::{Action, PrId};

use super::act::Origin;
use super::cmdline::{Completion, Editor};
use super::command::{self, Cmd, Resolved};
use super::{App, Effect, Filter, Input, PreviewMode};
use crate::keymap::Key;
use crate::span::markup;
use crate::views::chrome::Mode;
use crate::views::list::body_rows;

/// How long the first key of `gg`, `]f` or `zo` waits for the second.
pub const PENDING: Duration = Duration::from_millis(500);

impl App {
    /// Drops a half-typed sequence once it has waited too long.
    pub fn expire(&mut self, now: Instant) {
        if self.pending.is_some_and(|(_, at)| now.duration_since(at) >= PENDING) {
            self.pending = None;
        }
    }

    pub fn key(&mut self, key: Key, now: Instant) -> Vec<Effect> {
        self.expire(now);
        match std::mem::take(&mut self.input) {
            Input::Confirm(p) => {
                self.input = Input::Confirm(p);
                self.message = None;
                self.confirm_key(key)
            }
            Input::Command(e) => self.command_key(e, key),
            Input::CommandError { text, suggestion } => {
                self.message = None;
                match key {
                    Key::Esc => vec![],
                    Key::Tab if suggestion.is_some() => {
                        let rest = text.split_once(' ').map_or(String::new(), |(_, r)| format!(" {r}"));
                        self.input = Input::Command(Editor::with(&format!("{}{rest}", suggestion.unwrap_or_default())));
                        vec![]
                    }
                    k => self.command_key(Editor::with(&text), k),
                }
            }
            Input::Filter(e) => self.filter_key(e, key),
            Input::Help(f) => {
                self.help_key(f, key);
                vec![]
            }
            Input::Messages => {
                if !matches!(key, Key::Esc | Key::Char('q') | Key::Enter) {
                    self.input = Input::Messages;
                }
                vec![]
            }
            Input::Normal => {
                self.message = None;
                match self.detail_key(key, now) {
                    Some(effects) => effects,
                    None => self.normal_key(key, now),
                }
            }
        }
    }

    fn help_key(&mut self, mut f: String, key: Key) {
        match key {
            Key::Esc | Key::Char('?') => return,
            Key::Backspace => {
                f.pop();
            }
            Key::Char(c) => f.push(c),
            _ => {}
        }
        self.input = Input::Help(f);
    }

    fn rows(&self) -> usize {
        self.section().visible().len()
    }

    fn select(&mut self, i: usize) {
        let n = self.rows();
        self.section_mut().sel = i.min(n.saturating_sub(1));
    }

    fn switch(&mut self, to: usize) {
        if to < self.sections.len() {
            self.active = to;
        }
    }

    fn sel_id(&self) -> Option<PrId> {
        self.section().selected().map(|p| p.id.clone())
    }

    fn normal_key(&mut self, key: Key, now: Instant) -> Vec<Effect> {
        if let Some((first, _)) = self.pending.take() {
            return self.sequence(first, key);
        }
        let half = (body_rows(self.height) / 2).max(1);
        let sel = self.section().sel;
        let n = self.sections.len();
        match key {
            Key::Char('j') | Key::Down => self.select(sel + 1),
            Key::Char('k') | Key::Up => self.select(sel.saturating_sub(1)),
            Key::Char('G') => self.select(usize::MAX),
            Key::Ctrl('d') => self.select(sel + half),
            Key::Ctrl('u') => self.select(sel.saturating_sub(half)),
            Key::Char(c @ ('g' | ']' | '[' | 'z')) => self.pending = Some((c, now)),
            Key::Char('h') => self.switch((self.active + n - 1) % n),
            Key::Char('l') | Key::Tab => self.switch((self.active + 1) % n),
            Key::BackTab => self.switch((self.active + n - 1) % n),
            Key::Char(c @ '1'..='9') => self.switch(c as usize - '1' as usize),
            Key::Char(' ') => self.toggle_mark(),
            Key::Char('V') => self.mark_range(),
            Key::Char('/') => {
                let text = self.section().filter.as_ref().map(|f| f.text.clone()).unwrap_or_default();
                self.section_mut().filter = Some(Filter { text: text.clone(), applied: false });
                self.input = Input::Filter(Editor::with(&text));
            }
            Key::Char('n') => self.next_match(true),
            Key::Char('N') => self.next_match(false),
            Key::Char(':') => self.input = Input::Command(Editor::default()),
            Key::Char('?') => self.input = Input::Help(String::new()),
            Key::Char('p') => {
                let shown = self.preview_shown(self.width_hint());
                self.ui.preview = if shown { PreviewMode::Off } else { PreviewMode::On };
            }
            Key::Char('q') => self.quit = true,
            Key::Esc => {
                let s = self.section_mut();
                if s.filter.take().is_none() {
                    s.marks.clear();
                }
            }
            Key::Char('r') => return vec![Effect::Refresh { all: false }],
            Key::Char('R') => return vec![Effect::Refresh { all: true }],
            Key::Ctrl('l') => return vec![Effect::Redraw],
            Key::Char('M') => self.request_merge(self.targets(None), None, Origin::Key, None),
            Key::Char('A') => return self.request_arm(self.targets(None), None, Origin::Key, None),
            Key::Char('a') => return self.request_simple(self.targets(None), Action::Approve, Origin::Key, None),
            Key::Char('x') => return self.request_simple(self.targets(None), Action::Close, Origin::Key, None),
            Key::Char('u') => return self.request_simple(self.targets(None), Action::Update, Origin::Key, None),
            Key::Char('c') => {
                let targets = self.targets(None);
                if !targets.is_empty() {
                    return vec![Effect::Edit { targets }];
                }
            }
            Key::Char('o') => return self.urls(&self.targets(None)).into_iter().map(Effect::Open).collect(),
            Key::Enter => self.open_detail(false),
            Key::Char('d') => self.open_detail(true),
            _ => {}
        }
        vec![]
    }

    /// The width the preview decision was last made at. `p` flips what is
    /// on screen, so it needs the width the user is looking at.
    fn width_hint(&self) -> usize {
        self.last_width
    }

    fn sequence(&mut self, first: char, key: Key) -> Vec<Effect> {
        match (first, key) {
            ('g', Key::Char('g')) => self.select(0),
            (']' | '[', Key::Char('f' | 'c')) | ('z', Key::Char('o' | 'c')) => {
                self.refused("files, hunks and folds are in the detail view: Enter opens it")
            }
            _ => {}
        }
        vec![]
    }

    /// Space marks the row and moves on, as ranger does, so marking a run
    /// of rows is one key each.
    fn toggle_mark(&mut self) {
        let Some(id) = self.sel_id() else { return };
        let s = self.section_mut();
        match s.marks.iter().position(|m| *m == id) {
            Some(i) => {
                s.marks.remove(i);
            }
            None => s.marks.push(id),
        }
        let sel = s.sel;
        self.select(sel + 1);
    }

    /// V marks every row between the last mark and the cursor.
    fn mark_range(&mut self) {
        let s = self.section();
        let rows: Vec<PrId> = s.visible().iter().map(|p| p.id.clone()).collect();
        let anchor = s.marks.last().and_then(|m| rows.iter().position(|r| r == m)).unwrap_or(s.sel);
        let (a, b) = (anchor.min(s.sel), anchor.max(s.sel));
        let s = self.section_mut();
        for id in &rows[a..=b.min(rows.len().saturating_sub(1))] {
            if !s.marks.contains(id) {
                s.marks.push(id.clone());
            }
        }
    }

    fn next_match(&mut self, forward: bool) {
        let m = self.section().match_rows();
        if m.is_empty() || self.section().filter.is_none() {
            return;
        }
        let sel = self.section().sel;
        let next = if forward {
            m.iter().find(|i| **i > sel).or(m.first())
        } else {
            m.iter().rev().find(|i| **i < sel).or(m.last())
        };
        if let Some(i) = next.copied() {
            self.select(i);
        }
    }

    fn filter_key(&mut self, mut e: Editor, key: Key) -> Vec<Effect> {
        match key {
            Key::Esc => {
                self.section_mut().filter = None;
                return vec![];
            }
            Key::Enter => {
                let s = self.section_mut();
                let id = s.selected().map(|p| p.id.clone());
                if e.text.is_empty() {
                    s.filter = None;
                } else if let Some(f) = &mut s.filter {
                    f.applied = true;
                }
                let vis: Vec<PrId> = s.visible().iter().map(|p| p.id.clone()).collect();
                s.sel = id.and_then(|id| vis.iter().position(|v| *v == id)).unwrap_or(0);
                return vec![];
            }
            Key::Backspace => e.backspace(),
            Key::Delete => e.delete(),
            Key::Left => e.left(),
            Key::Right => e.right(),
            Key::Ctrl('w') => e.delete_word(),
            Key::Ctrl('u') => e.delete_line(),
            Key::Char(c) => e.insert(c),
            _ => {}
        }
        let text = e.text.clone();
        if let Some(f) = &mut self.section_mut().filter {
            f.text = text;
        }
        // Typing jumps to the first match from the top, so the cursor is
        // always on something the filter found.
        if let Some(first) = self.section().match_rows().first().copied() {
            self.select(first);
        }
        self.input = Input::Filter(e);
        vec![]
    }

    fn command_key(&mut self, mut e: Editor, key: Key) -> Vec<Effect> {
        if e.register {
            e.register = false;
            if key == Key::Char('#')
                && let Some(id) = self.sel_id()
            {
                e.insert_str(&format!("#{}", id.number));
            }
            self.input = Input::Command(e);
            return vec![];
        }
        match key {
            Key::Esc => return vec![],
            Key::Enter => return self.run_command(e.text),
            Key::Tab | Key::BackTab => self.complete(&mut e, key == Key::Tab),
            Key::Up => e.walk(&self.history, true),
            Key::Down => e.walk(&self.history, false),
            Key::Backspace if e.text.is_empty() => return vec![],
            Key::Backspace => e.backspace(),
            Key::Delete => e.delete(),
            Key::Left => e.left(),
            Key::Right => e.right(),
            Key::Home => e.home(),
            Key::End => e.end(),
            Key::Ctrl('w') => e.delete_word(),
            Key::Ctrl('u') => e.delete_line(),
            Key::Ctrl('r') => e.register = true,
            Key::Char(c) => e.insert(c),
            _ => {}
        }
        self.input = Input::Command(e);
        vec![]
    }

    fn complete(&mut self, e: &mut Editor, forward: bool) {
        if e.menu.is_some() {
            e.complete(Completion::Command, vec![], forward);
            return;
        }
        let (from, word) = e.word();
        let before: String = e.text.chars().take(from).collect();
        let cmd = command::lookup(before.split_whitespace().next().unwrap_or(""));
        if from == 0 {
            let items: Vec<String> =
                command::COMMANDS.iter().filter(|c| c.name.starts_with(&word)).map(|c| c.name.to_string()).collect();
            e.complete(Completion::Command, items, forward);
        } else if cmd == Some(Cmd::Section) {
            let w = word.to_lowercase();
            let items: Vec<String> = self
                .sections
                .iter()
                .flat_map(|s| [s.section.short.clone(), s.section.name.clone()])
                .filter(|n| n.to_lowercase().starts_with(&w) && !n.contains(' '))
                .collect();
            e.complete(Completion::Section, items, forward);
        } else if word.starts_with(['#', '!']) {
            let digits = &word[1..];
            let items: Vec<String> = self
                .section()
                .visible()
                .iter()
                .map(|p| p.id.number.to_string())
                .filter(|n| n.starts_with(digits))
                .map(|n| format!("#{n}"))
                .collect();
            e.complete(Completion::Target, items, forward);
        }
    }

    fn command_error(&mut self, typed: &str, text: &str, suggestion: Option<String>) -> Vec<Effect> {
        self.input = Input::CommandError { text: typed.into(), suggestion };
        self.error(text, Mode::Command);
        vec![]
    }

    fn run_command(&mut self, typed: String) -> Vec<Effect> {
        let line = typed.trim().to_string();
        if line.is_empty() {
            return vec![];
        }
        let mut effects = Vec::new();
        if self.history.last() != Some(&line) {
            self.history.push(line.clone());
            effects.push(Effect::SaveHistory);
        }
        let p = command::parse(&line);
        let Some(cmd) = command::lookup(p.name) else {
            let sug = command::did_you_mean(p.name);
            let mut msg = format!("{{r|E  not a command: :{}}}", super::act::esc(p.name));
            if let Some(s) = sug {
                msg.push_str(&format!("{{d|   did you mean :{s}?  Tab to fix}}"));
            }
            self.input = Input::CommandError { text: line, suggestion: sug.map(String::from) };
            self.say(markup(&msg), Mode::Command);
            return effects;
        };
        let explicit = match &p.target {
            None => None,
            Some(t) => {
                let rows: Vec<_> = self.section().visible().into_iter().cloned().collect();
                match command::resolve(t, &rows) {
                    Resolved::One(id) => Some(id),
                    Resolved::None => {
                        return self.command_error(&line, &format!("{}: no such PR in this section", p.name), None);
                    }
                    Resolved::Ambiguous(repos) => {
                        let msg =
                            format!("{}: that number is in {}; say which, as repo#number", p.name, repos.join(" and "));
                        return self.command_error(&line, &msg, None);
                    }
                }
            }
        };
        let ids = self.targets(explicit);
        let typed = Some(line.clone());
        let o = Origin::Command;
        let more = match cmd {
            Cmd::Merge => {
                let m = command::parse_method(p.rest.trim());
                self.request_merge(ids, m, o, typed);
                vec![]
            }
            Cmd::Automerge => match p.rest.trim() {
                "off" => {
                    let armed: Vec<PrId> =
                        ids.into_iter().filter(|id| self.find(id).is_some_and(|p| p.armed())).collect();
                    let jobs = armed.into_iter().map(|id| (id, Action::Disarm, false)).collect();
                    self.start(jobs, None, o, typed)
                }
                m => self.request_arm(ids, command::parse_method(m), o, typed),
            },
            Cmd::Approve => self.request_simple(ids, Action::Approve, o, typed),
            Cmd::Comment if p.rest.trim().is_empty() => vec![Effect::Edit { targets: ids }],
            Cmd::Comment => self.request_simple(ids, Action::Comment(p.rest.to_string()), o, typed),
            Cmd::Checkout => {
                let jobs = ids
                    .into_iter()
                    .filter_map(|id| self.find(&id).map(|p| (id, Action::Checkout { branch: p.head.clone() }, false)))
                    .collect();
                self.start(jobs, None, o, typed)
            }
            Cmd::Open => self.urls(&ids).into_iter().map(Effect::Open).collect(),
            Cmd::Close => self.request_simple(ids, Action::Close, o, typed),
            Cmd::Update => self.request_simple(ids, Action::Update, o, typed),
            Cmd::Diff => {
                if let Some(i) = ids.first().and_then(|id| self.section().visible().iter().position(|p| p.id == *id)) {
                    self.section_mut().sel = i;
                    self.open_detail(true);
                }
                vec![]
            }
            Cmd::Refresh => vec![Effect::Refresh { all: p.rest.trim() == "all" }],
            Cmd::Section => {
                let want = p.rest.trim().to_lowercase();
                match self
                    .sections
                    .iter()
                    .position(|s| s.section.short.to_lowercase() == want || s.section.name.to_lowercase() == want)
                {
                    Some(i) => self.active = i,
                    None => return self.command_error(&line, &format!("section: no section called {want}"), None),
                }
                vec![]
            }
            Cmd::Messages => {
                self.input = Input::Messages;
                vec![]
            }
            Cmd::Clear => {
                self.clear_merged();
                vec![]
            }
            Cmd::Quit | Cmd::QuitAll => {
                self.quit = true;
                vec![]
            }
        };
        effects.extend(more);
        effects
    }
}

#[cfg(test)]
mod tests;
