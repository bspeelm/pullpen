//! Glyph tiers (design §10). A tier swaps characters only; widths never
//! change, so layout code never asks which tier is active — except for the
//! forge tag, which is two letters in one tier and a glyph plus a space in
//! another.

use pullpen_core::Forge;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Glyphs {
    #[default]
    Unicode,
    Ascii,
    Nerd,
}

impl Glyphs {
    /// Every character a view emits passes through here on its way to the
    /// screen. Views always speak the Unicode tier.
    pub fn map(self, c: char) -> char {
        match self {
            Glyphs::Unicode => c,
            Glyphs::Ascii => ascii(c),
            Glyphs::Nerd => nerd(c),
        }
    }

    pub fn forge(self, f: Forge) -> &'static str {
        match (self, f) {
            // nf-fa-github / nf-fa-gitlab, then a space: one cell + space keeps
            // the column two wide.
            (Glyphs::Nerd, Forge::GitHub) => "\u{f09b} ",
            (Glyphs::Nerd, Forge::GitLab) => "\u{f296} ",
            _ => f.tag(),
        }
    }
}

/// tui.js `ASCII`.
fn ascii(c: char) -> char {
    match c {
        '✓' => '+',
        '✗' => 'x',
        '○' => 'o',
        '≠' => '!',
        '◌' => '-',
        '◆' => '*',
        '◇' => '^',
        '●' => '.',
        '▌' => '>',
        '│' => '|',
        '…' => '~',
        '─' => '-',
        '•' => '+',
        '→' => '>',
        '←' => '<',
        '›' => '>',
        '↑' => '^',
        '↓' => 'v',
        '·' => '-',
        '−' => '-',
        '–' => '-',
        '⠋' => '|',
        '⠙' => '/',
        '⠹' => '-',
        '⠸' => '\\',
        c => c,
    }
}

/// Nerd Font codepoints for the state glyphs; structure glyphs (cursor,
/// rule, ellipsis, spinner) are unchanged in this tier (design §10).
fn nerd(c: char) -> char {
    match c {
        '✓' => '\u{f42e}',  // nf-oct-check
        '○' => '\u{f43a}',  // nf-oct-clock
        '✗' => '\u{f467}',  // nf-oct-x
        '≠' => '\u{f421}',  // nf-oct-alert
        '◌' => '\u{f4dd}',  // nf-oct-git_pull_request_draft
        '●' => '\u{f419}',  // nf-oct-git_merge
        '◆' => '\u{f427}',  // nf-oct-rocket
        '◇' => '\u{f167a}', // nf-md-robot_outline
        c => c,
    }
}

/// The busy spinner, one frame per tick.
pub const SPINNER: [char; 4] = ['⠋', '⠙', '⠹', '⠸'];

#[cfg(test)]
mod tests;
