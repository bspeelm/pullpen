//! The one palette pullpen ships (ADR-003): ten tokens in three tiers.
//!
//! Values are the design's (`design/tui.js` `PAL`, spec §10). A test scans the
//! repository and fails on any colour value not defined here.

use ratatui::style::Color;

/// A colour role. Semantic tokens (`Ok`, `Warn`, `Err`) colour glyphs and
/// single keywords only, never whole rows (PLAN.md §5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tok {
    /// The terminal's own background. Never painted as a background; used as
    /// a foreground only for text on a filled block (the mode block).
    Bg,
    Surf,
    Sel,
    Fg,
    Dim,
    Faint,
    Acc,
    Ok,
    Warn,
    Err,
}

impl Tok {
    pub const ALL: [Tok; 10] =
        [Tok::Bg, Tok::Surf, Tok::Sel, Tok::Fg, Tok::Dim, Tok::Faint, Tok::Acc, Tok::Ok, Tok::Warn, Tok::Err];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    True,
    C256,
    C16,
    /// `NO_COLOR`: nothing is coloured, and glyphs and bold carry what
    /// colour would have (design §10).
    Mono,
}

/// Resolves tokens to terminal colours for one tier.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub tier: Tier,
    /// `ui.accent` from config, honoured in the truecolor tier only: the other
    /// tiers have no way to express an arbitrary colour.
    pub accent: Option<(u8, u8, u8)>,
    /// A palette file's colours, by token, replacing the built-in truecolor
    /// values (ADR-003).
    pub custom: Option<[(u8, u8, u8); 10]>,
}

impl Palette {
    pub fn new(tier: Tier) -> Self {
        Palette { tier, accent: None, custom: None }
    }

    /// `None` means "leave the terminal's default": surface and selection have
    /// no reliable shade in 16 colours, so selection falls back to bold plus
    /// the gutter mark instead.
    pub fn color(&self, t: Tok) -> Option<Color> {
        match self.tier {
            Tier::True => Some(match (t, self.accent) {
                (Tok::Acc, Some((r, g, b))) => Color::Rgb(r, g, b),
                _ => {
                    let (r, g, b) = self
                        .custom
                        .map_or_else(|| truecolor(t), |c| c[Tok::ALL.iter().position(|x| *x == t).unwrap_or(0)]);
                    Color::Rgb(r, g, b)
                }
            }),
            Tier::C256 => Some(Color::Indexed(c256(t))),
            Tier::C16 => c16(t),
            // The mode block and verdict label keep their shape as reverse
            // video, set by the painter; every other colour is the terminal's.
            Tier::Mono => None,
        }
    }
}

pub fn truecolor(t: Tok) -> (u8, u8, u8) {
    match t {
        Tok::Bg => (0x14, 0x15, 0x17),
        Tok::Surf => (0x1e, 0x1f, 0x23),
        Tok::Sel => (0x28, 0x2b, 0x31),
        Tok::Fg => (0xcb, 0xc9, 0xc4),
        Tok::Dim => (0x8a, 0x89, 0x84),
        Tok::Faint => (0x50, 0x50, 0x55),
        Tok::Acc => (0x7c, 0xc4, 0xdc),
        Tok::Ok => (0x94, 0xc9, 0x8b),
        Tok::Warn => (0xdc, 0xc0, 0x7a),
        Tok::Err => (0xe4, 0x8a, 0x7a),
    }
}

fn c256(t: Tok) -> u8 {
    match t {
        Tok::Bg => 233,
        Tok::Surf => 234,
        Tok::Sel => 236,
        Tok::Fg => 251,
        Tok::Dim => 245,
        Tok::Faint => 239,
        Tok::Acc => 116,
        Tok::Ok => 114,
        Tok::Warn => 180,
        Tok::Err => 174,
    }
}

fn c16(t: Tok) -> Option<Color> {
    match t {
        Tok::Bg => Some(Color::Black),
        Tok::Surf | Tok::Sel => None,
        Tok::Fg => Some(Color::Reset),
        Tok::Dim | Tok::Faint => Some(Color::DarkGray),
        Tok::Acc => Some(Color::Cyan),
        Tok::Ok => Some(Color::Green),
        Tok::Warn => Some(Color::Yellow),
        Tok::Err => Some(Color::Red),
    }
}

/// `#rrggbb`.
pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.strip_prefix('#')?;
    let n = u32::from_str_radix(h, 16).ok().filter(|_| h.len() == 6)?;
    Some(((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

/// A palette file: one `token = "#rrggbb"` line for each of the ten tokens,
/// named as `Tok` is, lower-case. All ten, or it is refused: a palette with a
/// hole would show the hole as an unreadable cell.
pub fn parse_file(text: &str) -> Result<[(u8, u8, u8); 10], String> {
    let mut out = [(0, 0, 0); 10];
    for (i, t) in Tok::ALL.iter().enumerate() {
        let name = format!("{t:?}").to_lowercase();
        let line = text
            .lines()
            .find_map(|l| l.split_once('=').filter(|(k, _)| k.trim() == name))
            .ok_or_else(|| format!("no {name} = \"#rrggbb\""))?;
        out[i] = parse_hex(line.1.trim().trim_matches('"')).ok_or_else(|| format!("{name} is not a #rrggbb colour"))?;
    }
    Ok(out)
}

/// The truecolor value as `#rrggbb`, the spelling the design and the config
/// file use.
pub fn hex(t: Tok) -> String {
    let (r, g, b) = truecolor(t);
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// The tier the terminal can show, from the environment the way most
/// terminals advertise it: `COLORTERM=truecolor` (or `24bit`), else a
/// `TERM` naming 256 colours, else 16.
pub fn detect(colorterm: Option<&str>, term: Option<&str>) -> Tier {
    if matches!(colorterm, Some("truecolor" | "24bit")) {
        Tier::True
    } else if term.is_some_and(|t| t.contains("256color")) {
        Tier::C256
    } else {
        Tier::C16
    }
}

#[cfg(test)]
mod tests;
