//! The rows every screen shares: section tabs on top, status line and
//! command line at the bottom (tui.js `tabs`, `status`).

use pullpen_core::Forge;

use crate::palette::Tok;
use crate::span::{Line, Span, lr, s, sp, with_bg};

pub struct Tab {
    pub name: String,
    pub short: String,
    pub count: usize,
}

/// tui.js `tabs`. Content starts at column 2 like every other row; the
/// section's query sits faint on the right.
pub fn tabs(tabs: &[Tab], active: usize, w: usize, short: bool, right: &str) -> Line {
    let mut l = vec![sp(2)];
    for (i, t) in tabs.iter().enumerate() {
        let name = if short { &t.short } else { &t.name };
        if i == active {
            l.push(s(name, Tok::Acc).bold());
            l.push(s(format!(" {}", t.count), Tok::Acc));
        } else {
            l.push(s(name, Tok::Dim));
            l.push(s(format!(" {}", t.count), Tok::Faint));
        }
        if i + 1 < tabs.len() {
            l.push(sp(if short { 3 } else { 4 }));
        }
    }
    let r = if right.is_empty() {
        vec![]
    } else {
        vec![s(right, Tok::Faint), sp(1)]
    };
    lr(&l, &r, w)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Command,
    Filter,
    Confirm,
}

impl Mode {
    fn word(self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Command => "COMMAND",
            Mode::Filter => "FILTER",
            Mode::Confirm => "CONFIRM",
        }
    }
    /// The mode block is accent, or warn while a confirmation is pending; it
    /// is never red (design §10).
    fn block(self) -> Tok {
        if self == Mode::Confirm {
            Tok::Warn
        } else {
            Tok::Acc
        }
    }
}

pub struct Status<'a> {
    pub mode: Mode,
    pub section: &'a str,
    pub pos: String,
    pub extra: Line,
    pub armed: usize,
    pub failing: usize,
    /// Below 100 columns the counts drop their words.
    pub narrow: bool,
    /// A background error. It replaces the sync segment and flips the
    /// failing forge's ✓ to ✗ until it clears.
    pub err: Option<(Forge, String)>,
    pub sync: String,
    pub forges: &'a [Forge],
}

/// tui.js `status`: where you are on the left, the world on the right.
pub fn status(w: usize, o: &Status) -> Line {
    let mut left = vec![
        Span::new(format!(" {} ", o.mode.word()), Tok::Bg)
            .bold()
            .on(o.mode.block()),
        sp(2),
        s(o.section, Tok::Fg),
        sp(2),
        s(&o.pos, Tok::Dim),
    ];
    if !o.extra.is_empty() {
        left.push(sp(3));
        left.extend(o.extra.iter().cloned());
    }
    let mut r = Vec::new();
    if let Some((_, e)) = &o.err {
        r.extend([s("E ", Tok::Err).bold(), s(e, Tok::Err), sp(3)]);
    }
    if o.narrow {
        r.extend([s("◆", Tok::Acc), s(o.armed.to_string(), Tok::Dim), sp(1)]);
        r.extend([s("✗", Tok::Err), s(o.failing.to_string(), Tok::Dim), sp(3)]);
    } else {
        r.extend([
            s("◆ ", Tok::Acc),
            s(format!("{} armed", o.armed), Tok::Dim),
            sp(3),
        ]);
        r.extend([
            s("✗ ", Tok::Err),
            s(format!("{} failing", o.failing), Tok::Dim),
            sp(3),
        ]);
    }
    if o.err.is_none() {
        r.extend([s(&o.sync, Tok::Dim), sp(3)]);
    }
    for (i, f) in o.forges.iter().enumerate() {
        if i > 0 {
            r.push(sp(2));
        }
        r.push(s(f.tag(), Tok::Dim));
        let down = o.err.as_ref().is_some_and(|(ef, _)| ef == f);
        r.push(if down {
            s(" ✗", Tok::Err)
        } else {
            s(" ✓", Tok::Ok)
        });
    }
    r.push(sp(1));
    with_bg(lr(&left, &r, w), Tok::Surf)
}
