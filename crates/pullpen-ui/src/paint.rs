//! Lines of spans onto a ratatui buffer: the one place tokens become
//! colours and Unicode glyphs become the active tier.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::glyphs::Glyphs;
use crate::palette::{Palette, Tier, Tok};
use crate::span::Line;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub palette: Palette,
    pub glyphs: Glyphs,
}

impl Theme {
    pub fn style(&self, fg: Tok, bg: Option<Tok>, bold: bool, underline: bool) -> Style {
        let mut st = Style::default().fg(self.palette.color(fg).unwrap_or(Color::Reset));
        let mut bold = bold;
        match bg.map(|b| (b, self.palette.color(b))) {
            Some((_, Some(c))) => st = st.bg(c),
            // Without colour, a filled label (mode block, verdict) is reverse
            // video, so it still reads as a label.
            Some((Tok::Acc | Tok::Warn | Tok::Ok | Tok::Err | Tok::Dim, None)) if self.palette.tier == Tier::Mono => {
                st = st.add_modifier(Modifier::REVERSED);
            }
            // No reliable selection shade in this tier: the row turns bold
            // and the gutter mark carries the rest (design §03).
            Some((Tok::Sel, None)) => bold = true,
            _ => st = st.bg(Color::Reset),
        }
        if bold {
            st = st.add_modifier(Modifier::BOLD);
        }
        if underline {
            // The underline takes the text's colour as its own, so terminals
            // that draw coloured underlines show the match in accent (design §13).
            st = st.add_modifier(Modifier::UNDERLINED);
            if let Some(c) = self.palette.color(fg) {
                st = st.underline_color(c);
            }
        }
        st
    }
}

/// Paints `lines` from the top of `area`, one per row, clipped to its width.
/// A short line is padded with plain spaces, as tui.js pads a row it left
/// empty. Rows beyond `lines` are left untouched.
pub fn paint(lines: &[Line], buf: &mut Buffer, area: Rect, theme: &Theme) {
    let pad = theme.style(Tok::Fg, None, false, false);
    let right = area.x + area.width;
    for (dy, line) in lines.iter().enumerate().take(area.height as usize) {
        let y = area.y + dy as u16;
        let mut x = area.x;
        for span in line {
            let st = theme.style(span.fg, span.bg, span.bold, span.underline);
            for c in span.text.chars().take((right - x) as usize) {
                buf[(x, y)].set_char(theme.glyphs.map(c)).set_style(st);
                x += 1;
            }
        }
        for x in x..right {
            buf[(x, y)].set_char(' ').set_style(pad);
        }
    }
}
