//! The PR list: column layout, rows, header (tui.js `cols`, `rowA`, `headA`,
//! `listScreen`).

use pullpen_core::time::age;
use pullpen_core::{PipelineState, Pr, Readiness};

use super::Ctx;
use super::chrome::{Status, Tab, status, tabs};
use crate::palette::Tok;
use crate::span::{Line, Span, fit, s, sp, with_bg};

/// Column widths for a list pane `w` cells wide. A zero width means the
/// column is dropped; `author == 2` means the agent mark alone and
/// `repo == 11` means the name after the slash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cols {
    pub forge: usize,
    pub repo: usize,
    pub author: usize,
    pub rev: usize,
    pub size: usize,
    pub age: usize,
    pub title: usize,
}

/// tui.js `cols(w)`: drop or shrink columns in a fixed order until the title
/// has at least 24 cells (design §11).
pub fn cols(w: usize) -> Cols {
    let mut c = Cols { forge: 3, repo: 17, author: 12, rev: 5, size: 12, age: 4, title: 0 };
    let sum = |c: &Cols| 12 + c.forge + c.repo + c.author + c.rev + c.size + c.age;
    let steps: [fn(&mut Cols); 7] = [
        |c| c.size = 0,
        |c| c.author = 2,
        |c| c.repo = 11,
        |c| c.age = 0,
        |c| c.rev = 0,
        |c| c.forge = 0,
        |c| c.repo = 0,
    ];
    for step in steps {
        if sum(&c) + 24 <= w {
            break;
        }
        step(&mut c);
    }
    c.title = w.saturating_sub(sum(&c));
    c
}

#[derive(Clone, Debug, Default)]
pub struct RowOpts<'a> {
    pub sel: bool,
    pub mark: bool,
    /// Lower-case filter text to underline in the title.
    pub hl: Option<&'a str>,
    /// Fade the whole row: a non-match while filtering.
    pub dim_all: bool,
    /// A spinner frame replacing the state glyph while an action runs.
    pub spin: Option<char>,
}

pub fn state_glyph(r: Readiness) -> Span {
    match r {
        Readiness::Merged => s("●", Tok::Dim),
        Readiness::Draft => s("◌", Tok::Dim),
        Readiness::Conflict => s("≠", Tok::Err),
        Readiness::Failing => s("✗", Tok::Err),
        Readiness::Waiting => s("○", Tok::Warn),
        Readiness::Ready => s("✓", Tok::Ok),
    }
}

/// The state glyph, or for a merged PR its pipeline's: ○ running, ✓
/// passed, ✗ failed (ADR-017).
pub fn glyph(p: &Pr, r: Readiness) -> Span {
    match p.post_merge.as_ref().filter(|_| p.merged()).map(|x| x.state) {
        Some(PipelineState::Running) => s("○", Tok::Warn),
        Some(PipelineState::Passed) => s("✓", Tok::Ok),
        Some(PipelineState::Failed) => s("✗", Tok::Err),
        None => state_glyph(r),
    }
}

fn rev(p: &Pr) -> Span {
    let (h, n) = (p.reviews.have, p.reviews.need);
    if n == 0 { s("–", Tok::Faint) } else { s(format!("{h}/{n}"), if h >= n { Tok::Ok } else { Tok::Warn }) }
}

pub fn gutter(sel: bool, mark: bool) -> Span {
    if sel {
        s("▌ ", Tok::Acc)
    } else if mark {
        s("• ", Tok::Acc)
    } else {
        sp(2)
    }
}

/// The title, with the first match of `hl` underlined in accent. Matching is
/// ASCII case-insensitive, which keeps byte offsets valid in any title.
pub fn title_segs(title: &str, hl: Option<&str>, fg: Tok) -> Line {
    let Some(hl) = hl.filter(|h| !h.is_empty()) else {
        return vec![s(title, fg)];
    };
    let Some(i) = title.to_ascii_lowercase().find(&hl.to_ascii_lowercase()) else {
        return vec![s(title, fg)];
    };
    let j = i + hl.len();
    vec![s(&title[..i], fg), s(&title[i..j], Tok::Acc).underline(), s(&title[j..], fg)]
}

pub fn pad_start(text: &str, w: usize) -> String {
    format!("{}{text}", " ".repeat(w.saturating_sub(text.chars().count())))
}

pub fn pad_end(text: &str, w: usize) -> String {
    format!("{text}{}", " ".repeat(w.saturating_sub(text.chars().count())))
}

pub fn agent_mark(p: &Pr) -> Span {
    if p.author.is_agent { s("◇ ", Tok::Fg) } else { sp(2) }
}

/// tui.js `rowA`.
pub fn row(p: &Pr, w: usize, o: &RowOpts, ctx: &Ctx) -> Line {
    let c = cols(w);
    let mut l = vec![gutter(o.sel, o.mark)];
    if c.forge > 0 {
        l.push(s(pad_end(ctx.glyphs.forge(p.id.forge), c.forge), Tok::Dim));
    }
    l.push(match o.spin {
        Some(f) => s(f.to_string(), Tok::Acc),
        None => glyph(p, ctx.readiness(p)),
    });
    l.push(sp(1));
    l.push(if p.armed() { s("◆", Tok::Acc) } else { sp(1) });
    l.push(sp(1));
    l.push(s(pad_start(&format!("#{}", p.id.number), 5) + " ", if o.sel { Tok::Fg } else { Tok::Dim }));
    let tf = if p.draft || p.merged() { Tok::Dim } else { Tok::Fg };
    let title: Line = title_segs(&p.title, o.hl, tf)
        .into_iter()
        .map(|sp| if o.sel && sp.fg == Tok::Fg { sp.bold() } else { sp })
        .collect();
    l.extend(fit(&title, c.title.saturating_sub(2), true));
    l.push(sp(2));
    if c.repo > 0 {
        let r = if c.repo == 11 { p.id.repo_name() } else { &p.id.repo };
        l.extend(fit(&[s(r, Tok::Dim)], c.repo - 1, true));
        l.push(sp(1));
    }
    if c.author == 12 {
        l.extend(fit(&[agent_mark(p), s(&p.author.login, Tok::Dim)], 11, true));
        l.push(sp(1));
    } else if c.author == 2 {
        l.push(agent_mark(p));
    }
    if c.rev > 0 {
        l.extend(fit(&[rev(p)], c.rev - 1, true));
        l.push(sp(1));
    }
    if c.size > 0 {
        let size =
            format!("{} {} ", pad_start(&format!("+{}", p.additions), 5), pad_end(&format!("−{}", p.deletions), 5));
        l.push(s(size, Tok::Dim));
    }
    if c.age > 0 {
        l.push(s(pad_start(&age(p.created_at, ctx.now), 4), Tok::Dim));
    }
    let mut l = fit(&l, w, false);
    if o.dim_all {
        for sp in &mut l {
            sp.fg = Tok::Faint;
            sp.bold = false;
        }
    }
    if o.sel { with_bg(l, Tok::Sel) } else { l }
}

/// tui.js `headA`.
pub fn header(w: usize) -> Line {
    let c = cols(w);
    let mut l = vec![sp(2)];
    if c.forge > 0 {
        l.push(sp(c.forge));
    }
    l.push(sp(4));
    l.push(s("    #", Tok::Faint));
    l.push(sp(1));
    l.extend(fit(&[s("title", Tok::Faint)], c.title.saturating_sub(2), true));
    l.push(sp(2));
    if c.repo > 0 {
        l.extend(fit(&[s("repo", Tok::Faint)], c.repo, true));
    }
    if c.author == 12 {
        l.extend(fit(&[sp(2), s("author", Tok::Faint)], 12, true));
    } else if c.author == 2 {
        l.push(sp(2));
    }
    if c.rev > 0 {
        l.extend(fit(&[s("rev", Tok::Faint)], c.rev, true));
    }
    if c.size > 0 {
        l.extend(fit(&[s("  +/−", Tok::Faint)], c.size, true));
    }
    if c.age > 0 {
        l.push(s(" age", Tok::Faint));
    }
    fit(&l, w, false)
}

/// The preview pane's width, and the gap before it (` │ `, plus the one-cell
/// right margin).
pub const PREVIEW_W: usize = 46;

pub struct ListScreen<'a> {
    pub w: usize,
    pub h: usize,
    pub tabs: &'a [Tab],
    pub active: usize,
    pub short: bool,
    pub right: &'a str,
    pub prs: &'a [Pr],
    /// One per row of `prs`.
    pub opts: &'a [RowOpts<'a>],
    /// First row shown; the caller keeps the selection inside the window.
    pub offset: usize,
    pub preview: Option<Vec<Line>>,
    pub status: Status<'a>,
    pub cmd: Line,
}

/// How many PR rows fit under the header in an `h`-row screen.
pub fn body_rows(h: usize) -> usize {
    h.saturating_sub(5)
}

/// tui.js `listScreen`.
pub fn list_screen(o: &ListScreen, ctx: &Ctx) -> Vec<Line> {
    let (w, h) = (o.w, o.h);
    let mut out = vec![tabs(o.tabs, o.active, w, o.short, o.right), Vec::new()];
    let body_h = h.saturating_sub(4);
    let lw = if o.preview.is_some() { w.saturating_sub(PREVIEW_W + 4) } else { w.saturating_sub(1) };
    let mut rows = vec![header(lw)];
    for (p, ro) in o.prs.iter().zip(o.opts).skip(o.offset).take(body_h.saturating_sub(1)) {
        rows.push(row(p, lw, ro, ctx));
    }
    for i in 0..body_h {
        let r = rows.get(i).cloned().unwrap_or_default();
        match &o.preview {
            Some(pv) => {
                let mut l = fit(&r, lw, false);
                l.push(s(" │ ", Tok::Faint));
                l.extend(fit(pv.get(i).map_or(&[][..], |v| v), PREVIEW_W, true));
                l.push(sp(1));
                out.push(l);
            }
            None => out.push(fit(&r, w, false)),
        }
    }
    out.push(status(w, &o.status));
    out.push(fit(&o.cmd, w, true));
    out
}

/// The row strips the design uses for the merge, batch and command-line
/// examples (tui.js `strip`): rows at `w - 1`, then status and command line.
pub fn strip(prs: &[Pr], opts: &[RowOpts], w: usize, status_o: &Status, cmd: &Line, ctx: &Ctx) -> Vec<Line> {
    let mut out: Vec<Line> = prs.iter().zip(opts).map(|(p, o)| fit(&row(p, w - 1, o, ctx), w, false)).collect();
    out.push(status(w, status_o));
    out.push(fit(cmd, w, true));
    out
}

#[cfg(test)]
mod tests;
