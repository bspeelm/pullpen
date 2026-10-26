//! A buffer as text, in the format `scripts/gen-golden.mjs` writes, so the
//! renderer can be compared with the design byte for byte (ADR-002).
//!
//! `plain` is the characters; `styled` prefixes each run of one style with
//! `{fg [bg=x] [b] [u]}`. Truecolor runs are named by token; 16-colour runs
//! by the colour asked for, because two tokens share a colour there.

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

use crate::palette::{Tier, Tok, truecolor};

pub fn serialize(buf: &Buffer, tier: Tier) -> (String, String) {
    let area = buf.area;
    let (mut plain, mut styled) = (String::new(), String::new());
    for y in area.y..area.y + area.height {
        let mut last = String::new();
        for x in area.x..area.x + area.width {
            let cell = &buf[(x, y)];
            let mut tag = name(cell.fg, tier, false).unwrap_or_else(|| "?".into());
            if let Some(bg) = name(cell.bg, tier, true) {
                tag.push_str(&format!(" bg={bg}"));
            }
            if cell.modifier.contains(Modifier::BOLD) {
                tag.push_str(" b");
            }
            if cell.modifier.contains(Modifier::UNDERLINED) {
                tag.push_str(" u");
            }
            if tag != last {
                styled.push_str(&format!("{{{tag}}}"));
                last = tag;
            }
            plain.push_str(cell.symbol());
            styled.push_str(cell.symbol());
        }
        plain.push('\n');
        styled.push('\n');
    }
    (plain, styled)
}

fn name(c: Color, tier: Tier, is_bg: bool) -> Option<String> {
    let n = match (tier, c) {
        (_, Color::Reset) if is_bg => return None,
        (Tier::True, Color::Rgb(r, g, b)) => Tok::ALL
            .iter()
            .find(|t| truecolor(**t) == (r, g, b))
            .map(|t| format!("{t:?}").to_lowercase())?,
        (Tier::C16, Color::Reset) => "default".into(),
        (Tier::C16, Color::Black) => "black".into(),
        (Tier::C16, Color::DarkGray) => "brightblack".into(),
        (Tier::C16, Color::Cyan) => "cyan".into(),
        (Tier::C16, Color::Green) => "green".into(),
        (Tier::C16, Color::Yellow) => "yellow".into(),
        (Tier::C16, Color::Red) => "red".into(),
        (_, Color::Indexed(i)) => format!("idx{i}"),
        (_, other) => format!("{other:?}"),
    };
    Some(n)
}
