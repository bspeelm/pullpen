//! The Auto-merge armed view (tui.js `armedScreen`, `rowArm`): armed PRs
//! grouped by what they need, each with the reason on the right.

use pullpen_core::time::{Timestamp, age};
use pullpen_core::{Mergeable, PipelineState, Pr, PrState, Readiness};

use super::Ctx;
use super::list::{glyph, gutter, pad_start};
use crate::palette::Tok;
use crate::span::{Line, fit, markup, s, sp, with_bg};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    Attention,
    Waiting,
    Merged,
}

/// Which group a PR belongs in. `disarmed` means pullpen saw it armed and
/// the forge has since turned auto-merge off without being asked to.
pub fn group(p: &Pr, r: Readiness, disarmed: bool) -> Option<Group> {
    match () {
        _ if p.merged() => Some(Group::Merged),
        _ if disarmed && !p.armed() => Some(Group::Attention),
        _ if !p.armed() => None,
        _ if matches!(r, Readiness::Failing | Readiness::Conflict) || p.reviews.changes_requested => {
            Some(Group::Attention)
        }
        _ => Some(Group::Waiting),
    }
}

/// The reason column, as markup.
pub fn why(p: &Pr, g: Group, disarmed: bool, others: &[Pr]) -> String {
    let e = super::detail::esc;
    match g {
        Group::Merged => match &p.state {
            PrState::Merged { method, .. } => format!("{{d|merged · {}}}", method.word()),
            _ => "{d|merged}".into(),
        },
        Group::Attention => {
            if disarmed && !p.armed() {
                return "{r|disarmed}{d|  by the forge}".into();
            }
            if let Some(c) = p.checks.failed.first() {
                let detail = if c.detail.is_empty() { String::new() } else { format!(" · {}", e(&c.detail)) };
                return format!("{{r|failing}}{{d|  {}{detail}}}", e(&c.name));
            }
            if p.mergeable == Mergeable::Conflict {
                // The likeliest cause is the latest merge into the same branch.
                let after = others
                    .iter()
                    .filter(|o| o.id.repo == p.id.repo && o.base == p.base && o.id != p.id)
                    .filter_map(
                        |o| if let PrState::Merged { at, .. } = o.state { Some((at, o.id.number)) } else { None },
                    )
                    .max()
                    .map_or(String::new(), |(_, n)| format!(" after #{n}"));
                return format!("{{r|conflict}}{{d|  with {}{after}}}", e(&p.base));
            }
            "{r|changes}{d|  requested}".into()
        }
        Group::Waiting => {
            let (c, r) = (&p.checks, &p.reviews);
            if let Some(t) = p.merge_train {
                return format!("{{y|merge train}}{{d|  position {} of {}}}", t.position, t.length);
            }
            if !c.all_passed() {
                if c.running == 1 && c.queued.len() == 1 {
                    return format!("{{y|ci}}{{d|  required check {} queued}}", e(&c.queued[0]));
                }
                let then = if r.short() { ", then review" } else { "" };
                let eta = c.eta_mins.map_or(String::new(), |m| format!(" · ~{m}m left"));
                return format!("{{y|ci}}{{d|  {}/{} running{then}{eta}}}", c.required_passed, c.required_total);
            }
            if r.short() {
                let who: Vec<String> = r.pending.iter().map(|p| format!("@{}", e(p))).collect();
                let who = if who.is_empty() { String::new() } else { format!("{} · ", who.join(" ")) };
                return format!("{{y|review}}{{d|  {who}{} of {}}}", r.have, r.need);
            }
            "{y|merge}{d|  queued on the forge}".into()
        }
    }
}

/// The Merged view's groups (ADR-017), by what the merge commit's
/// pipeline did on the base branch. A failure needs attention until the
/// branch passes again (ADR-019).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Landed {
    Attention,
    Running,
    Done,
}

pub fn landed(p: &Pr) -> Landed {
    match p.post_merge.as_ref().map(|x| (x.state, x.branch)) {
        Some((PipelineState::Failed, Some(PipelineState::Passed))) => Landed::Done,
        Some((PipelineState::Failed, _)) => Landed::Attention,
        Some((PipelineState::Running, _)) => Landed::Running,
        _ => Landed::Done,
    }
}

/// The Merged view's reason column, as markup.
pub fn landed_why(p: &Pr) -> String {
    let e = super::detail::esc;
    let Some(pl) = &p.post_merge else { return "{d|no pipeline}".into() };
    let count = format!("{}/{}", pl.passed, pl.total);
    let base = e(&p.base);
    match (pl.state, pl.job.as_str()) {
        _ if landed(p) == Landed::Done && pl.state == PipelineState::Failed => {
            format!("{{g|fixed}}{{d|  {base} passes now}}")
        }
        (PipelineState::Failed, job) if pl.branch == Some(PipelineState::Failed) => {
            format!("{{r|{base} failing}}{{d|  {}}}", e(job))
        }
        (PipelineState::Failed, "") => format!("{{r|failed}}{{d|  {count}}}"),
        (PipelineState::Failed, job) => format!("{{r|failed}}{{d|  {}}}", e(job)),
        (PipelineState::Running, "") => format!("{{y|running}}{{d|  {count}}}"),
        (PipelineState::Running, job) => format!("{{y|running}}{{d|  {count} · {}}}", e(job)),
        (PipelineState::Passed, _) => format!("{{g|passed}}{{d|  {count}}}"),
    }
}

const RW: usize = 34;

/// tui.js `rowArm`. `lit` keeps a merged PR's title bright, for the Merged
/// view, where every row is merged and dimming them all would say nothing.
pub fn row(p: &Pr, w: usize, sel: bool, why: &str, when: Timestamp, lit: bool, ctx: &Ctx) -> Line {
    let tw = w.saturating_sub(2 + 3 + 2 + 2 + 6 + 12 + RW + 5);
    let merged = p.merged();
    let mut title = s(&p.title, if merged && !lit { Tok::Dim } else { Tok::Fg });
    if sel {
        title = title.bold();
    }
    let mut l = vec![
        gutter(sel, false),
        s(format!("{} ", ctx.glyphs.forge(p.id.forge)), Tok::Dim),
        glyph(p, ctx.readiness(p)),
        sp(1),
        if merged { sp(1) } else { s("◆", Tok::Acc) },
        sp(1),
        s(pad_start(&format!("#{}", p.id.number), 5) + " ", if sel { Tok::Fg } else { Tok::Dim }),
    ];
    l.extend(fit(&[title], tw.saturating_sub(2), true));
    l.push(sp(2));
    // One cell is kept clear, so a long name cannot run into the reason.
    l.extend(fit(&[s(p.id.repo_name(), Tok::Dim)], 11, true));
    l.push(sp(1));
    l.extend(fit(&markup(why), RW, true));
    l.push(s(pad_start(&age(when, ctx.now), 5), Tok::Dim));
    let l = fit(&l, w, false);
    if sel { with_bg(l, Tok::Sel) } else { l }
}

/// One group's heading: lowercase, bold, with its count.
pub fn heading(title: &str, n: usize) -> Line {
    vec![sp(2), s(title, Tok::Fg).bold(), s(format!("  {n}"), Tok::Dim)]
}

/// `09:14`, in the local zone given as an offset from UTC.
pub fn clock(t: Timestamp, offset: i64) -> String {
    let m = (t.0 + offset).rem_euclid(86_400) / 60;
    format!("{:02}:{:02}", m / 60, m % 60)
}
