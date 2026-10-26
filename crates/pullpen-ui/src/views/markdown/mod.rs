//! PR descriptions and comments are markdown. This draws the parts that
//! carry meaning in a terminal (headings, emphasis, lists, task boxes,
//! quotes, code) and drops the syntax, rather than showing `**` and `[x]`.
//! Not a full CommonMark parser: what it does not recognise stays as text.

use crate::palette::Tok;
use crate::span::{Line, Span, s, sp};

/// Inline markup to spans: `**bold**`, `__bold__`, `` `code` ``,
/// `[text](url)` and `![alt](url)`; single `*` and `_` emphasis is left
/// as written, since snake_case and globs use the same characters.
pub fn inline(text: &str, base: Tok) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let flush = |plain: &mut String, out: &mut Vec<Span>| {
        if !plain.is_empty() {
            out.push(s(std::mem::take(plain), base));
        }
    };
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let close = |marker: &str| {
            rest.strip_prefix(marker).and_then(|r| r.find(marker).map(|i| (&r[..i], &r[i + marker.len()..])))
        };
        if let Some((inner, after)) = close("**").or_else(|| close("__")).filter(|(i, _)| !i.is_empty()) {
            flush(&mut plain, &mut out);
            out.extend(inline(inner, base).into_iter().map(Span::bold));
            rest = after;
        } else if let Some((inner, after)) = close("`").filter(|(i, _)| !i.is_empty()) {
            plain.push_str(inner);
            rest = after;
        } else if let Some((img, label, after)) = link(rest) {
            if img {
                flush(&mut plain, &mut out);
                let alt = if label.is_empty() { String::new() } else { format!(": {label}") };
                out.push(s(format!("[image{alt}]"), Tok::Dim));
            } else {
                plain.push_str(label);
            }
            rest = after;
        } else if let Some(after) = tag(rest) {
            rest = after;
        } else {
            plain.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    flush(&mut plain, &mut out);
    out
}

/// `[label](url)` or `![alt](url)`: whether it is an image, the label, and
/// what follows.
fn link(text: &str) -> Option<(bool, &str, &str)> {
    let (img, body) = match text.strip_prefix("![") {
        Some(b) => (true, b),
        None => (false, text.strip_prefix('[')?),
    };
    let end = body.find("](")?;
    let after = &body[end + 2..];
    let close = after.find(')')?;
    Some((img, &body[..end], &after[close + 1..]))
}

/// An HTML tag such as `<br>`, `<details open>`, `</summary>`: dropped. A
/// `<` followed by a space, as in `a < b`, is not a tag.
fn tag(text: &str) -> Option<&str> {
    let body = text.strip_prefix('<')?;
    let end = body.find('>').filter(|e| *e < 60)?;
    body[..end].trim_start_matches('/').chars().next().filter(char::is_ascii_alphabetic)?;
    Some(&body[end + 1..])
}

/// Greedy word wrap over styled spans, with `indent` cells before every line
/// after the first: the same breaks `span::wrap` makes for plain text, so a
/// plain paragraph renders exactly as it did before markdown was read.
pub fn wrap(spans: &[Span], w: usize, indent: usize) -> Vec<Line> {
    let mut words: Vec<Vec<Span>> = vec![vec![]];
    for sp in spans {
        let mut first = true;
        for part in sp.text.split(' ') {
            if !first {
                words.push(vec![]);
            }
            first = false;
            if !part.is_empty()
                && let Some(wd) = words.last_mut()
            {
                wd.push(Span { text: part.into(), ..sp.clone() })
            }
        }
    }
    let width = |wd: &[Span]| wd.iter().map(|x| x.text.chars().count()).sum::<usize>();
    let mut out: Vec<Line> = Vec::new();
    let (mut line, mut used): (Line, usize) = (Vec::new(), 0);
    for wd in words.into_iter().filter(|wd| !wd.is_empty()) {
        let n = width(&wd);
        let room = if out.is_empty() { w } else { w.saturating_sub(indent) };
        if used > 0 && used + 1 + n > room {
            out.push(std::mem::take(&mut line));
            used = 0;
        }
        if used > 0 {
            line.push(sp(1));
            used += 1;
        } else if !out.is_empty() && indent > 0 {
            line.push(sp(indent));
        }
        used += n;
        line.extend(wd);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn list_item(l: &str) -> Option<(String, &str)> {
    let t = l.trim_start();
    for b in ["- ", "* ", "+ "] {
        if let Some(rest) = t.strip_prefix(b) {
            return Some(match rest.get(..4) {
                Some("[x] " | "[X] ") => ("{g|✓} ".into(), &rest[4..]),
                Some("[ ] ") => ("{x|○} ".into(), &rest[4..]),
                _ => ("{d|-} ".into(), rest),
            });
        }
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    let rest = t.get(digits..)?.strip_prefix(". ").filter(|_| digits > 0)?;
    Some((format!("{{d|{}.}} ", &t[..digits]), rest))
}

/// A whole description or comment, `w` cells wide.
pub fn render(text: &str, w: usize) -> Vec<Line> {
    let text = strip_comments(&text.replace("\r\n", "\n"));
    let mut out: Vec<Line> = Vec::new();
    let mut para: Vec<&str> = Vec::new();
    let mut fence = false;
    let blank = |out: &mut Vec<Line>| {
        if out.last().is_some_and(|l| !l.is_empty()) {
            out.push(vec![]);
        }
    };
    let flush = |para: &mut Vec<&str>, out: &mut Vec<Line>| {
        if !para.is_empty() {
            out.extend(wrap(&inline(&para.join(" "), Tok::Fg), w, 0));
            para.clear();
        }
    };
    for l in text.lines() {
        let t = l.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            flush(&mut para, &mut out);
            fence = !fence;
            continue;
        }
        if fence {
            out.push(vec![sp(2), s(l.trim_end(), Tok::Dim)]);
            continue;
        }
        if t.is_empty() {
            flush(&mut para, &mut out);
            blank(&mut out);
        } else if let Some(h) =
            t.strip_prefix('#').map(|h| h.trim_start_matches('#')).filter(|h| h.starts_with(' ') || h.is_empty())
        {
            flush(&mut para, &mut out);
            out.extend(wrap(&inline(h.trim(), Tok::Fg).into_iter().map(Span::bold).collect::<Vec<_>>(), w, 0));
        } else if t.chars().all(|c| matches!(c, '-' | '*' | '_')) && t.len() >= 3 {
            flush(&mut para, &mut out);
            out.push(vec![s("─".repeat(w.min(40)), Tok::Faint)]);
        } else if let Some((mark, rest)) = list_item(l) {
            flush(&mut para, &mut out);
            let depth = (l.len() - l.trim_start().len()).min(8);
            let mut spans = vec![sp(depth)];
            spans.extend(crate::span::markup(&mark));
            let prefix: usize = spans.iter().map(|x| x.text.chars().count()).sum();
            spans.extend(inline(rest, Tok::Fg));
            out.extend(wrap_keeping(&spans, w, prefix));
        } else if let Some(q) = t.strip_prefix('>') {
            flush(&mut para, &mut out);
            for mut line in wrap(&inline(q.trim(), Tok::Dim), w.saturating_sub(2), 0) {
                line.insert(0, s("│ ", Tok::Faint));
                out.push(line);
            }
        } else if t.starts_with('|') {
            flush(&mut para, &mut out);
            out.push(vec![s(t, Tok::Dim)]);
        } else {
            para.push(t);
        }
    }
    flush(&mut para, &mut out);
    while out.last().is_some_and(Vec::is_empty) {
        out.pop();
    }
    out
}

/// `wrap`, keeping leading spaces (list indentation) that word splitting
/// would otherwise collapse.
fn wrap_keeping(spans: &[Span], w: usize, indent: usize) -> Vec<Line> {
    let lead: usize = spans.first().filter(|x| x.text.trim().is_empty()).map_or(0, |x| x.text.len());
    let rest = if lead > 0 { &spans[1..] } else { spans };
    let mut lines = wrap(rest, w.saturating_sub(lead), indent.saturating_sub(lead));
    if lead > 0 {
        for l in &mut lines {
            l.insert(0, sp(lead));
        }
    }
    lines
}

fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("<!--") {
        out.push_str(&rest[..i]);
        rest = rest[i..].find("-->").map_or("", |j| &rest[i + j + 3..]);
    }
    out.push_str(rest);
    out
}

/// The text of the first prose paragraph, with markup removed: what the
/// preview, which has room for a few lines, shows.
pub fn first_paragraph(text: &str) -> String {
    let rendered = render(text, usize::MAX / 2);
    let mut words = Vec::new();
    for l in rendered.iter() {
        if l.is_empty() {
            if !words.is_empty() {
                break;
            }
            continue;
        }
        let t: String = l.iter().map(|x| x.text.as_str()).collect();
        let is_prose = l.first().is_some_and(|x| x.fg == Tok::Fg && !x.bold && !x.text.starts_with(' '));
        if !is_prose {
            if !words.is_empty() {
                break;
            }
            continue;
        }
        words.push(t);
    }
    words.join(" ")
}

#[cfg(test)]
mod tests;
