//! The line editor behind `:` and `/`: cursor, history, completion.

use crate::keymap::Key;
use crate::palette::Tok;
use crate::span::{Line, Span, s};

/// What a completion menu offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Completion {
    Command,
    Target,
    Section,
}

#[derive(Clone, Debug)]
pub struct Menu {
    pub kind: Completion,
    /// The text each item puts in place of the word being completed.
    pub items: Vec<String>,
    pub sel: usize,
    /// Char index where the completed word starts.
    pub from: usize,
}

#[derive(Clone, Debug)]
pub struct HistoryWalk {
    /// What was typed before walking, which filters the walk and comes back
    /// when walking off the newest end.
    pub typed: String,
    /// Indices into the history, newest first, of entries starting with
    /// `typed`.
    pub matches: Vec<usize>,
    /// `None` before the first step, and after walking back off the newest.
    pub pos: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct Editor {
    pub text: String,
    /// In chars, not bytes.
    pub cursor: usize,
    pub menu: Option<Menu>,
    pub walk: Option<HistoryWalk>,
    /// `^r` was pressed and the next key says what to insert.
    pub register: bool,
}

impl Editor {
    pub fn with(text: &str) -> Self {
        Editor { text: text.into(), cursor: text.chars().count(), ..Default::default() }
    }

    fn byte(&self, ch: usize) -> usize {
        self.text.char_indices().nth(ch).map_or(self.text.len(), |(b, _)| b)
    }

    /// Any edit ends completion and history walking: what is on the line is
    /// now the user's.
    fn edited(&mut self) {
        self.menu = None;
        self.walk = None;
    }

    pub fn insert(&mut self, c: char) {
        let b = self.byte(self.cursor);
        self.text.insert(b, c);
        self.cursor += 1;
        self.edited();
    }

    pub fn insert_str(&mut self, t: &str) {
        for c in t.chars() {
            self.insert(c);
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let b = self.byte(self.cursor);
            self.text.remove(b);
        }
        self.edited();
    }

    pub fn delete(&mut self) {
        if self.cursor < self.text.chars().count() {
            let b = self.byte(self.cursor);
            self.text.remove(b);
        }
        self.edited();
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.text.chars().count());
    }
    pub fn home(&mut self) {
        self.cursor = 0;
    }
    pub fn end(&mut self) {
        self.cursor = self.text.chars().count();
    }

    /// `^w`: the word before the cursor, and the spaces after it.
    pub fn delete_word(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = self.cursor;
        while i > 0 && chars[i - 1] == ' ' {
            i -= 1;
        }
        while i > 0 && chars[i - 1] != ' ' {
            i -= 1;
        }
        let (a, b) = (self.byte(i), self.byte(self.cursor));
        self.text.replace_range(a..b, "");
        self.cursor = i;
        self.edited();
    }

    /// The keys every line edits the same way: cursor, deletion, typing.
    pub fn edit(&mut self, key: Key) {
        match key {
            Key::Backspace => self.backspace(),
            Key::Delete => self.delete(),
            Key::Left => self.left(),
            Key::Right => self.right(),
            Key::Home => self.home(),
            Key::End => self.end(),
            Key::Ctrl('w') => self.delete_word(),
            Key::Ctrl('u') => self.delete_line(),
            Key::Char(c) => self.insert(c),
            _ => {}
        }
    }

    /// `^u`: everything before the cursor.
    pub fn delete_line(&mut self) {
        let b = self.byte(self.cursor);
        self.text.replace_range(..b, "");
        self.cursor = 0;
        self.edited();
    }

    /// The word under completion: from the last space before the cursor.
    pub fn word(&self) -> (usize, String) {
        let before: String = self.text.chars().take(self.cursor).collect();
        let from = before.rfind(' ').map_or(0, |b| before[..b].chars().count() + 1);
        (from, before.chars().skip(from).collect())
    }

    /// Replaces the word under completion with `with`, keeping the menu.
    pub fn fill(&mut self, from: usize, with: &str) {
        let (a, b) = (self.byte(from), self.byte(self.cursor));
        self.text.replace_range(a..b, with);
        self.cursor = from + with.chars().count();
    }

    /// Starts or advances a completion. `items` is only used when no menu is
    /// open; an open one cycles.
    pub fn complete(&mut self, kind: Completion, items: Vec<String>, forward: bool) {
        if let Some(m) = &mut self.menu {
            let n = m.items.len();
            m.sel = if forward { (m.sel + 1) % n } else { (m.sel + n - 1) % n };
            let (from, item) = (m.from, m.items[m.sel].clone());
            self.fill(from, &item);
            return;
        }
        let (from, _) = self.word();
        match items.len() {
            0 => {}
            1 => {
                self.fill(from, &items[0]);
            }
            _ => {
                self.fill(from, &items[0]);
                self.menu = Some(Menu { kind, items, sel: 0, from });
            }
        }
    }

    /// ↑ (`older`) or ↓ through `history` (oldest first), filtered by the
    /// prefix typed before the walk began.
    pub fn walk(&mut self, history: &[String], older: bool) {
        let mut w = self.walk.take().unwrap_or_else(|| HistoryWalk {
            typed: self.text.clone(),
            matches: (0..history.len()).rev().filter(|i| history[*i].starts_with(&self.text)).collect(),
            pos: None,
        });
        w.pos = match (older, w.pos) {
            (true, None) if w.matches.is_empty() => None,
            (true, None) => Some(0),
            (true, Some(p)) => Some((p + 1).min(w.matches.len() - 1)),
            (false, None | Some(0)) => None,
            (false, Some(p)) => Some(p - 1),
        };
        self.text = match w.pos {
            Some(p) => history[w.matches[p]].clone(),
            None => w.typed.clone(),
        };
        self.cursor = self.text.chars().count();
        self.menu = None;
        self.walk = w.pos.is_some().then_some(w);
    }

    /// `history 3/14` while walking.
    pub fn walk_label(&self) -> Option<String> {
        let w = self.walk.as_ref()?;
        Some(format!("history {}/{}", w.pos? + 1, w.matches.len()))
    }

    /// The prefix and the text, with the cursor as an inverted cell.
    pub fn line(&self, prefix: char) -> Line {
        let mut out = vec![s(prefix.to_string(), Tok::Fg)];
        let chars: Vec<char> = self.text.chars().collect();
        let before: String = chars[..self.cursor].iter().collect();
        if !before.is_empty() {
            out.push(s(before, Tok::Fg));
        }
        let at = chars.get(self.cursor).map_or(' ', |c| *c);
        out.push(Span::new(at.to_string(), Tok::Bg).on(Tok::Fg));
        if self.cursor + 1 < chars.len() {
            out.push(s(chars[self.cursor + 1..].iter().collect::<String>(), Tok::Fg));
        }
        out
    }
}

#[cfg(test)]
mod tests;
