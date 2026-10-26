//! The mouse, additive only (design §13): nothing here that a key cannot do.

use std::time::{Duration, Instant};

use super::{App, Input};
use crate::views::detail::Pane;
use crate::views::list::body_rows;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mouse {
    Click { x: u16, y: u16 },
    Scroll { x: u16, down: bool },
}

/// Two clicks on one row this close together open it.
const DOUBLE: Duration = Duration::from_millis(400);
/// Rows above the list body: tabs, a blank, the header.
const LIST_TOP: u16 = 3;

impl App {
    pub fn mouse(&mut self, m: Mouse, now: Instant) {
        if !matches!(self.input, Input::Normal) {
            return;
        }
        if let Some(d) = &mut self.detail {
            if let Mouse::Scroll { x, down } = m {
                let p = if usize::from(x) < 2 + 56 + 2 && !d.zoom { Pane::Overview } else { Pane::Diff };
                let s = &mut d.scroll[p as usize];
                *s = if down { *s + 3 } else { s.saturating_sub(3) };
            }
            return;
        }
        match m {
            Mouse::Click { x, y: 0 } => {
                if let Some(i) = self.tab_at(usize::from(x)) {
                    self.active = i;
                }
            }
            Mouse::Click { y, .. } if y >= LIST_TOP && usize::from(y - LIST_TOP) < body_rows(self.height) => {
                if self.sections[self.active].section.view == pullpen_core::View::Armed {
                    return;
                }
                let i = self.section().offset + usize::from(y - LIST_TOP);
                if i >= self.section().visible().len() {
                    return;
                }
                let double = self.last_click.is_some_and(|(j, at)| j == i && now.duration_since(at) < DOUBLE);
                self.section_mut().sel = i;
                self.last_click = Some((i, now));
                if double {
                    self.open_detail(false);
                }
            }
            Mouse::Scroll { down, .. } => {
                let n = self.section().visible().len();
                let s = self.section_mut();
                s.sel = if down { (s.sel + 3).min(n.saturating_sub(1)) } else { s.sel.saturating_sub(3) };
            }
            Mouse::Click { .. } => {}
        }
    }

    /// Which tab covers column `x`, laid out as `views::chrome::tabs` lays
    /// them out.
    fn tab_at(&self, x: usize) -> Option<usize> {
        let short = self.narrow(self.last_width);
        let mut at = 2;
        for (i, s) in self.sections.iter().enumerate() {
            let name = if short { &s.section.short } else { &s.section.name };
            let w = name.chars().count() + 1 + s.total.to_string().len();
            if x >= at && x < at + w {
                return Some(i);
            }
            at += w + if short { 3 } else { 4 };
        }
        None
    }
}

#[cfg(test)]
mod tests;
