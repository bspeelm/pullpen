//! Spans and the four primitives every screen is built from: `fit`, `lr`,
//! `overlay` and the `markup` parser. Ported from `design/tui.js` (`S`, `fit`,
//! `lr`, `overlay`, `M`) and kept behaviourally identical, because the golden
//! tests compare their output byte for byte (ADR-002).
//!
//! Widths are counted in chars, as tui.js counts code points: every glyph the
//! design uses is one cell (PLAN.md §5.1).

use crate::palette::Tok;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub fg: Tok,
    pub bg: Option<Tok>,
    pub bold: bool,
    pub underline: bool,
}

pub type Line = Vec<Span>;

impl Span {
    pub fn new(text: impl Into<String>, fg: Tok) -> Self {
        Span {
            text: text.into(),
            fg,
            bg: None,
            bold: false,
            underline: false,
        }
    }
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
    pub fn on(mut self, bg: Tok) -> Self {
        self.bg = Some(bg);
        self
    }
    fn with_text(&self, text: impl Into<String>) -> Self {
        Span {
            text: text.into(),
            ..self.clone()
        }
    }
}

/// `S(t, f)`.
pub fn s(text: impl Into<String>, fg: Tok) -> Span {
    Span::new(text, fg)
}

/// `sp(n)`: plain spaces, no background of their own, so a later `with_bg`
/// fills them.
pub fn sp(n: usize) -> Span {
    Span::new(" ".repeat(n), Tok::Fg)
}

pub fn width(l: &[Span]) -> usize {
    l.iter().map(|s| s.text.chars().count()).sum()
}

/// Pads or truncates to exactly `w` cells. With `ellipsis`, a truncated line
/// ends in `…` styled like the text it replaces; trailing spaces before the
/// cut move after the ellipsis, so a truncated column keeps its right gap.
pub fn fit(l: &[Span], w: usize, ellipsis: bool) -> Line {
    let n = width(l);
    if n <= w {
        let mut out = l.to_vec();
        if n < w {
            out.push(sp(w - n));
        }
        return out;
    }
    if w == 0 {
        return Vec::new();
    }
    let lim = if ellipsis { w - 1 } else { w };
    let mut out: Line = Vec::new();
    let mut c = 0;
    for s in l {
        let len = s.text.chars().count();
        if c + len <= lim {
            out.push(s.clone());
            c += len;
        } else {
            let k = lim - c;
            if k > 0 {
                out.push(s.with_text(s.text.chars().take(k).collect::<String>()));
            }
            break;
        }
    }
    if ellipsis {
        let last = out.last().cloned().unwrap_or_else(|| s("", Tok::Fg));
        let mut moved = 0;
        if last.text.ends_with(' ') {
            let trimmed = last.text.trim_end_matches(' ').to_string();
            moved = last.text.len() - trimmed.len();
            if let Some(l) = out.last_mut() {
                l.text = trimmed;
            }
        }
        out.push(Span {
            text: "…".into(),
            underline: false,
            ..last
        });
        if moved > 0 {
            out.push(sp(moved));
        }
    }
    out
}

/// Gives every span without a background this one.
pub fn with_bg(l: Line, bg: Tok) -> Line {
    l.into_iter()
        .map(|mut s| {
            if s.bg.is_none() {
                s.bg = Some(bg);
            }
            s
        })
        .collect()
}

/// `a` left-justified and `b` right-justified in `w` cells; `a` is truncated
/// with an ellipsis when they would overlap.
pub fn lr(a: &[Span], b: &[Span], w: usize) -> Line {
    let mut out = fit(a, w.saturating_sub(width(b)), true);
    out.extend_from_slice(b);
    out
}

/// Writes `top` over `base` starting at cell `x`, cell by cell, then merges
/// runs of identical style back into spans.
pub fn overlay(base: &[Span], x: usize, top: &[Span]) -> Line {
    let mut cells = to_cells(base);
    for (i, c) in to_cells(top).into_iter().enumerate() {
        let at = x + i;
        if at < cells.len() {
            cells[at] = c;
        } else {
            cells.push(c);
        }
    }
    from_cells(cells)
}

fn to_cells(l: &[Span]) -> Vec<Span> {
    l.iter()
        .flat_map(|s| s.text.chars().map(move |c| s.with_text(c)))
        .collect()
}

fn from_cells(cells: Vec<Span>) -> Line {
    let mut out: Line = Vec::new();
    for c in cells {
        match out.last_mut() {
            Some(p)
                if p.fg == c.fg
                    && p.bg == c.bg
                    && p.bold == c.bold
                    && p.underline == c.underline =>
            {
                p.text.push_str(&c.text)
            }
            _ => out.push(c),
        }
    }
    out
}

/// The design's markup, `M("{aB|text}")`. Inside the braces each letter sets
/// the colour (`f d x a g r y` = fg dim faint acc ok err warn) or a flag
/// (`B` bold, `U` underline, `C` cursor block, `S` selection background).
/// Text outside braces is plain fg.
pub fn markup(src: &str) -> Line {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some((open, tags, body, end)) = next_tag(rest) {
        if open > 0 {
            out.push(s(&rest[..open], Tok::Fg));
        }
        let mut span = s(body, Tok::Fg);
        for t in tags.chars() {
            match t {
                'f' => span.fg = Tok::Fg,
                'd' => span.fg = Tok::Dim,
                'x' => span.fg = Tok::Faint,
                'a' => span.fg = Tok::Acc,
                'g' => span.fg = Tok::Ok,
                'r' => span.fg = Tok::Err,
                'y' => span.fg = Tok::Warn,
                'B' => span.bold = true,
                'U' => span.underline = true,
                'C' => {
                    span.bg = Some(Tok::Fg);
                    span.fg = Tok::Bg;
                }
                'S' => span.bg = Some(Tok::Sel),
                _ => {}
            }
        }
        out.push(span);
        rest = &rest[end..];
    }
    if !rest.is_empty() {
        out.push(s(rest, Tok::Fg));
    }
    out
}

/// The next `{tags|body}` in `src`, as tui.js's `/\{([a-zA-Z]+)\|([^}]*)\}/`
/// finds it: a brace that does not open a well-formed tag is plain text.
fn next_tag(src: &str) -> Option<(usize, &str, &str, usize)> {
    let mut from = 0;
    while let Some(i) = src[from..].find('{').map(|i| from + i) {
        let after = &src[i + 1..];
        let tags_len = after.bytes().take_while(u8::is_ascii_alphabetic).count();
        if tags_len > 0 && after.as_bytes().get(tags_len) == Some(&b'|') {
            let body_start = i + 1 + tags_len + 1;
            if let Some(close) = src[body_start..].find('}') {
                let body_end = body_start + close;
                return Some((
                    i,
                    &src[i + 1..i + 1 + tags_len],
                    &src[body_start..body_end],
                    body_end + 1,
                ));
            }
        }
        from = i + 1;
    }
    None
}

/// Greedy word wrap at `w` chars, as tui.js `wrap` does: words are never
/// split, so a word longer than `w` gets a line to itself.
pub fn wrap(text: &str, w: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        if cur.is_empty() {
            cur = word.to_string();
        } else if cur.chars().count() + 1 + word.chars().count() <= w {
            cur.push(' ');
            cur.push_str(word);
        } else {
            out.push(std::mem::replace(&mut cur, word.to_string()));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests;
