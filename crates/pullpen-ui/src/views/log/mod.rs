//! A job's log as lines of spans: ANSI colours mapped onto the palette,
//! everything else a terminal would act on dropped, and the forges' own
//! markers read for what they say (ADR-020).

use pullpen_core::time::{Timestamp, parse_rfc3339};

use crate::palette::Tok;
use crate::span::{Line, Span, s};

/// Lines kept; a longer log keeps its tail.
pub const MAX_LINES: usize = 10_000;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Log {
    pub lines: Vec<LogLine>,
    /// GitLab's collapsible sections, which stand in for steps.
    pub sections: Vec<Section>,
    pub trimmed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogLine {
    pub spans: Line,
    /// The plain text, for search.
    pub text: String,
    pub at: Option<Timestamp>,
    pub error: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub line: usize,
    pub started: Timestamp,
    pub finished: Option<Timestamp>,
}

/// `gitlab` turns on GitLab's per-line stream prefix (`00O `, or `00O+`
/// for a line continuing the one before).
pub fn parse(raw: &str, gitlab: bool) -> Log {
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let mut logical: Vec<(Option<Timestamp>, String)> = Vec::new();
    for l in raw.lines() {
        let (at, rest) = stamp(l);
        let (more, rest) = if gitlab { stream(rest) } else { (false, rest) };
        match logical.last_mut() {
            Some(prev) if more => prev.1.push_str(rest),
            _ => logical.push((at, rest.to_string())),
        }
    }
    let mut log = Log::default();
    // Each section's marker name, which its end repeats; `name` becomes the
    // header a person reads.
    let mut keys: Vec<String> = Vec::new();
    for (at, content) in logical {
        let (content, marks) = markers(&content);
        for (start, epoch, name) in marks {
            if start {
                log.sections.push(Section {
                    name: name.clone(),
                    line: log.lines.len(),
                    started: Timestamp(epoch),
                    finished: None,
                });
                keys.push(name);
            } else if let Some(i) = keys.iter().rposition(|k| *k == name) {
                log.sections[i].finished = Some(Timestamp(epoch));
            }
        }
        let opening = log.sections.last().is_some_and(|x| x.line == log.lines.len() && x.finished.is_none());
        let Some(line) = render(&content, at) else { continue };
        if line.text.trim().is_empty() && (opening || content.contains("section_")) {
            continue;
        }
        if opening && let Some(sec) = log.sections.last_mut() {
            sec.name = line.text.trim().to_string();
        }
        log.lines.push(line);
    }
    if log.lines.len() > MAX_LINES {
        let cut = log.lines.len() - MAX_LINES;
        log.lines.drain(..cut);
        log.sections.retain(|x| x.line >= cut);
        log.sections.iter_mut().for_each(|x| x.line -= cut);
        log.trimmed = true;
    }
    log
}

/// The RFC 3339 time both forges put before every line, when there is one.
fn stamp(l: &str) -> (Option<Timestamp>, &str) {
    let b = l.as_bytes();
    if b.len() < 21 || b[4] != b'-' || b[10] != b'T' {
        return (None, l);
    }
    match l.split_once(' ') {
        Some((t, rest)) => match parse_rfc3339(t) {
            Some(at) => (Some(at), rest),
            None => (None, l),
        },
        None => (None, l),
    }
}

fn stream(l: &str) -> (bool, &str) {
    let b = l.as_bytes();
    if b.len() >= 4
        && b[..2].iter().all(u8::is_ascii_hexdigit)
        && matches!(b[2], b'O' | b'E')
        && matches!(b[3], b' ' | b'+')
    {
        (b[3] == b'+', &l[4..])
    } else {
        (false, l)
    }
}

/// GitLab's `section_start:<epoch>:<name>[options]\r\e[0K`, and its end,
/// taken out of the line and returned in order.
fn markers(content: &str) -> (String, Vec<(bool, i64, String)>) {
    let mut out = String::new();
    let mut marks = Vec::new();
    let mut rest = content;
    loop {
        let found = [("section_start:", true), ("section_end:", false)]
            .into_iter()
            .filter_map(|(m, start)| rest.find(m).map(|i| (i, m, start)))
            .min_by_key(|(i, ..)| *i);
        let Some((i, m, start)) = found else { break };
        out.push_str(&rest[..i]);
        let after = &rest[i + m.len()..];
        let end = after.find('\r').unwrap_or(after.len());
        let body = &after[..end];
        let (epoch, name) = body.split_once(':').unwrap_or((body, ""));
        let name = name.split('[').next().unwrap_or_default();
        marks.push((start, epoch.parse().unwrap_or(0), name.to_string()));
        rest = after[end..].trim_start_matches('\r');
        rest = rest.strip_prefix("\x1b[0K").unwrap_or(rest);
    }
    out.push_str(rest);
    (out, marks)
}

/// One line as spans, or `None` for a marker that shows nothing.
fn render(content: &str, at: Option<Timestamp>) -> Option<LogLine> {
    let (tag, content) = match content.strip_prefix("##[") {
        Some(rest) => match rest.split_once(']') {
            Some(("endgroup", _)) => return None,
            Some((t, body)) => (Some(t), body),
            None => (None, content),
        },
        None => (None, content),
    };
    // A carriage return redraws the line, as a progress bar does; what
    // stands at the end is what a terminal would show.
    let content = content.rsplit('\r').find(|seg| !seg.is_empty()).unwrap_or("");
    let base = match tag {
        Some("group" | "command") => Some(Tok::Acc),
        Some("error") => Some(Tok::Err),
        Some("warning") => Some(Tok::Warn),
        Some("debug") => Some(Tok::Dim),
        _ => None,
    };
    let spans = sgr(content, base);
    let text: String = spans.iter().map(|x| x.text.as_str()).collect();
    let t = text.trim_start();
    let error = spans.iter().any(|x| x.fg == Tok::Err)
        || ["ERROR:", "error:", "error[", "Error:"].iter().any(|p| t.starts_with(p));
    Some(LogLine { spans, text, at, error })
}

/// Colours from SGR sequences, onto the palette's tokens. Backgrounds are
/// ignored, since the terminal's own is never painted over.
fn sgr(content: &str, base: Option<Tok>) -> Line {
    let mut out: Line = Vec::new();
    let (mut fg, mut bold) = (base, false);
    let mut cur = String::new();
    let flush = |cur: &mut String, out: &mut Line, fg: Option<Tok>, bold: bool| {
        if !cur.is_empty() {
            let mut sp: Span = s(std::mem::take(cur), fg.unwrap_or(Tok::Fg));
            if bold {
                sp = sp.bold();
            }
            out.push(sp);
        }
    };
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => {
                match chars.next() {
                    Some('[') => {}
                    // An operating-system command (a title, a link) runs to
                    // BEL or to ESC \.
                    Some(']') => {
                        while let Some(p) = chars.next() {
                            if p == '\x07' || (p == '\x1b' && chars.next_if_eq(&'\\').is_some()) {
                                break;
                            }
                        }
                        continue;
                    }
                    _ => continue,
                }
                let mut params = String::new();
                let mut fin = None;
                for p in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&p) {
                        fin = Some(p);
                        break;
                    }
                    params.push(p);
                }
                if fin != Some('m') {
                    continue;
                }
                flush(&mut cur, &mut out, fg, bold);
                let mut codes = params.split(';').map(|x| x.parse::<u16>().unwrap_or(0));
                while let Some(code) = codes.next() {
                    match code {
                        0 => (fg, bold) = (base, false),
                        1 => bold = true,
                        22 => bold = false,
                        30 | 90 => fg = Some(Tok::Dim),
                        31 | 91 => fg = Some(Tok::Err),
                        32 | 92 => fg = Some(Tok::Ok),
                        33 | 93 => fg = Some(Tok::Warn),
                        34 | 35 | 36 | 94 | 95 | 96 => fg = Some(Tok::Acc),
                        37 | 97 | 39 => fg = base,
                        // 256-colour and truecolour forms carry their own
                        // arguments, which are not codes.
                        38 | 48 => match codes.next() {
                            Some(5) => {
                                codes.next();
                            }
                            Some(2) => {
                                codes.nth(2);
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            '\t' => {
                let col: usize = out.iter().map(|x| x.text.chars().count()).sum::<usize>() + cur.chars().count();
                cur.push_str(&" ".repeat(8 - col % 8));
            }
            c if c.is_control() => {}
            c => cur.push(c),
        }
    }
    flush(&mut cur, &mut out, fg, bold);
    out
}

#[cfg(test)]
mod tests;
