//! Keys, and the one table that documents them. The `?` overlay and
//! `docs/keymap.md` are both generated from `ENTRIES`; a test fails if either
//! disagrees with it or if a bound key has no entry.

/// A key press, independent of the terminal library that read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
}

/// Where an entry is shown in the `?` overlay: one of six columns in two
/// blocks of three (design §08).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Move,
    Act,
    Sections,
    Detail,
    Command,
    Confirm,
    /// Bound and documented, but not in the overlay, which has room for the
    /// keys people look up rather than all of them.
    More,
}

impl Group {
    pub fn heading(self) -> &'static str {
        match self {
            Group::Move => "move",
            Group::Act => "act on selection",
            Group::Sections => "sections & views",
            Group::Detail => "detail view",
            Group::Command => "command line",
            Group::Confirm => "confirm prompt",
            Group::More => "more",
        }
    }
    pub const OVERLAY: [[Group; 3]; 2] =
        [[Group::Move, Group::Act, Group::Sections], [Group::Detail, Group::Command, Group::Confirm]];
}

pub struct Entry {
    pub keys: &'static str,
    pub what: &'static str,
    pub group: Group,
}

const fn e(keys: &'static str, what: &'static str, group: Group) -> Entry {
    Entry { keys, what, group }
}

/// Every key pullpen binds, in overlay order. `1–N` is rewritten to the
/// number of sections when shown.
pub const ENTRIES: &[Entry] = &[
    e("j k", "down / up", Group::Move),
    e("gg G", "top / bottom", Group::Move),
    e("^d ^u", "half page", Group::Move),
    e("/", "filter list", Group::Move),
    e("n N", "next / prev match", Group::Move),
    e("Esc", "clear, cancel", Group::Move),
    e("Space", "mark row", Group::Move),
    e("V", "mark range", Group::Move),
    e("Enter", "open detail", Group::Move),
    e("M", "merge…", Group::Act),
    e("A", "arm / disarm auto", Group::Act),
    e("a", "approve", Group::Act),
    e("c", "comment", Group::Act),
    e("d", "open diff", Group::Act),
    e("o", "open in browser", Group::Act),
    e("x", "close…", Group::Act),
    e("r", "refresh", Group::Act),
    e("h l", "prev / next section", Group::Sections),
    e("Tab S-Tab", "next / prev section", Group::Sections),
    e("1–N", "go to section", Group::Sections),
    e("p", "toggle preview", Group::Sections),
    e(":", "command line", Group::Sections),
    e("?", "this help", Group::Sections),
    e("q", "back / quit", Group::Sections),
    e("h l", "focus pane", Group::Detail),
    e("]f [f", "next / prev file", Group::Detail),
    e("]c [c", "next / prev hunk", Group::Detail),
    e("zo zc", "expand / fold", Group::Detail),
    e("q Esc", "back to list", Group::Detail),
    e("Tab S-Tab", "complete", Group::Command),
    e("↑ ↓", "history", Group::Command),
    e("^r #", "insert selected", Group::Command),
    e("Enter", "run", Group::Command),
    e("Esc", "cancel", Group::Command),
    e("y Enter", "confirm", Group::Confirm),
    e("n Esc", "cancel", Group::Confirm),
    e("s m r", "pick method", Group::Confirm),
    e("R", "refresh every section", Group::More),
    e("^l", "redraw", Group::More),
    e("^w ^u", "delete word / line (command line)", Group::More),
    e("d", "zoom the diff (detail)", Group::More),
    e("j k ^d ^u gg G", "scroll the focused pane (detail)", Group::More),
];

/// The display form of an entry's keys for `n` sections.
pub fn keys_for(e: &Entry, sections: usize) -> String {
    e.keys.replace("1–N", &format!("1–{}", sections.clamp(1, 9)))
}

/// `docs/keymap.md`, generated. `make keymap` writes it; a test compares.
pub fn markdown() -> String {
    let mut out = String::from(
        "# Keys\n\nGenerated from `crates/pullpen-ui/src/keymap/mod.rs` by `make keymap`; edit the table, not this file.\nThe same table drives the `?` overlay.\n",
    );
    let groups = [Group::Move, Group::Act, Group::Sections, Group::Detail, Group::Command, Group::Confirm, Group::More];
    for g in groups {
        out.push_str(&format!("\n## {}\n\n| key | does |\n|---|---|\n", g.heading()));
        for en in ENTRIES.iter().filter(|x| x.group == g) {
            out.push_str(&format!("| `{}` | {} |\n", en.keys.replace('|', "\\|"), en.what));
        }
    }
    out.push_str("\nAny other key at a confirm prompt cancels it and is discarded.\n");
    out
}

#[cfg(test)]
mod tests;
