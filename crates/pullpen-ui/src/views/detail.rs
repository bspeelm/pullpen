//! The detail view (tui.js `detailScreen`): verdict on top, the facts behind
//! it, then overview and diff side by side.

use pullpen_core::diff::{FileDiff, Kind};
use pullpen_core::time::age;
use pullpen_core::*;

use super::Ctx;
use super::chrome::{Status, status};
use super::list::pad_start;
use super::words::verdict_long;
use crate::palette::Tok;
use crate::span::{Line, Span, fit, lr, markup, s, sp};

const LW: usize = 56;
const FACT_W: usize = 59;
/// Checks shown before the rest fold behind `zo`.
const FOLDED: usize = 5;
/// Rows above the panes: breadcrumb, title, meta, verdict, facts, heading.
const TOP: usize = 11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Overview,
    Diff,
}

#[derive(Clone, Debug)]
pub struct Detail {
    pub id: PrId,
    pub focus: Pane,
    /// First row shown, per pane.
    pub scroll: [usize; 2],
    pub file: usize,
    pub zoom: bool,
    pub expanded: bool,
    /// The file list `F` opens: what has been typed to filter it, and the
    /// highlighted row.
    pub picker: Option<(String, usize)>,
}

impl Detail {
    pub fn new(id: PrId) -> Self {
        Detail { id, focus: Pane::Overview, scroll: [0, 0], file: 0, zoom: false, expanded: false, picker: None }
    }
}

pub struct DetailScreen<'a> {
    pub w: usize,
    pub h: usize,
    pub pr: &'a Pr,
    pub detail: &'a PrDetail,
    pub files: Option<&'a [FileDiff]>,
    pub d: &'a Detail,
    pub section: &'a str,
    /// `1 of 23`.
    pub position: String,
    pub status: Status<'a>,
    pub cmd: Line,
}

pub fn esc(t: &str) -> String {
    t.replace('{', "｛").replace('}', "｝")
}

pub fn body_rows(h: usize) -> usize {
    h.saturating_sub(TOP + 2)
}

fn label(v: &Verdict, pr: &Pr) -> Span {
    let (word, bg) = match (&pr.state, v) {
        (PrState::Merged { .. }, _) => (" MERGED ", Tok::Dim),
        (PrState::Closed, _) => (" CLOSED ", Tok::Dim),
        (_, Verdict::Ready) => (" READY ", Tok::Ok),
        (_, Verdict::Waiting(_)) => (" WAITING ", Tok::Warn),
        (_, Verdict::Blocked(_)) => (" BLOCKED ", Tok::Err),
    };
    s(word, Tok::Bg).bold().on(bg)
}

fn facts(pr: &Pr, d: &PrDetail, ctx: &Ctx) -> [String; 4] {
    let c = &pr.checks;
    let took = if d.checks_took.is_empty() { String::new() } else { format!("{{d|   {}}}", d.checks_took) };
    let checks = match c.failed.first() {
        Some(f) => format!("{{r|✗}}  {{d|checks      }}{{f|{} failing}}", esc(&f.name)),
        None if c.running > 0 || c.required_passed < c.required_total => {
            format!("{{y|○}}  {{d|checks      }}{{f|{}/{} running}}", c.required_passed, c.required_total)
        }
        None if c.required_total == 0 => "{g|✓}  {d|checks      }{f|none required}".into(),
        None => format!("{{g|✓}}  {{d|checks      }}{{f|{}/{} passed}}{took}", c.required_passed, c.required_total),
    };
    let r = &pr.reviews;
    let who: Vec<String> = r
        .approved_by
        .iter()
        .map(|a| format!("{} ✓", esc(a)))
        .chain(r.pending.iter().map(|p| format!("{} pending", esc(p))))
        .collect();
    let who = if who.is_empty() { String::new() } else { format!("{{d|   {}}}", who.join("   ")) };
    let reviews = match (r.changes_requested, r.need) {
        (true, _) => format!("{{r|✗}}  {{d|reviews     }}{{f|changes requested}}{who}"),
        (_, 0) => format!("{{g|✓}}  {{d|reviews     }}{{f|none required}}{who}"),
        _ => {
            let g = if r.short() { "{y|○}" } else { "{g|✓}" };
            format!("{g}  {{d|reviews     }}{{f|{} of {} required}}{who}", r.have, r.need)
        }
    };
    let base = esc(&pr.base);
    let conflicts = match pr.mergeable {
        Mergeable::Clean => format!("{{g|✓}}  {{d|conflicts   }}{{f|none}}{{d|   up to date with {base}}}"),
        Mergeable::Behind => format!("{{g|✓}}  {{d|conflicts   }}{{f|none}}{{d|   behind {base}}}"),
        Mergeable::Conflict => format!("{{r|≠}}  {{d|conflicts   }}{{f|with {base}}}"),
        Mergeable::Unknown => "{y|○}  {d|conflicts   }{f|checking}".into(),
    };
    let auto = match &pr.auto_merge {
        Some(a) => format!(
            "{{a|◆}}  {{d|auto-merge  }}{{f|armed}}{{d|   {} · by {} {} ago}}",
            a.method.word(),
            esc(&a.armed_by),
            age(a.armed_at, ctx.now)
        ),
        None => "{d|·}  {d|auto-merge  }{f|off}{d|   A arms}".into(),
    };
    [checks, reviews, conflicts, auto]
}

fn check_line(c: &CheckLine) -> Line {
    let g = match c.state {
        CheckState::Passed => "{g|✓}",
        CheckState::Failed => "{r|✗}",
        CheckState::Running | CheckState::Queued => "{y|○}",
    };
    lr(&markup(&format!("{g} {}", esc(&c.name))), &markup(&format!("{{d|{}}}", esc(&c.duration))), LW)
}

fn activity(a: &Activity, ctx: &Ctx) -> Vec<Line> {
    let who = if a.is_agent { format!("◇ {}", esc(&a.who)) } else { esc(&a.who) };
    let (g, what) = match &a.kind {
        ActivityKind::Approved => ("{g|✓}", "approved".to_string()),
        ActivityKind::ChangesRequested => ("{r|✗}", "requested changes".into()),
        ActivityKind::ArmedAutoMerge => ("{a|◆}", "armed auto-merge".into()),
        ActivityKind::DisarmedAutoMerge => ("{d|·}", "disarmed auto-merge".into()),
        ActivityKind::Pushed { commits: 1 } => ("{d|·}", "pushed 1 commit".into()),
        ActivityKind::Pushed { commits } => ("{d|·}", format!("pushed {commits} commits")),
        ActivityKind::ReviewRequested { by: Some(b) } => ("{y|○}", format!("review requested by {}", esc(b))),
        ActivityKind::ReviewRequested { by: None } => ("{y|○}", "review requested".into()),
        ActivityKind::Commented => ("{d|·}", "commented".into()),
    };
    let mut out = vec![lr(
        &markup(&format!("{g} {{f|{who}}}{{d|  {what}}}")),
        &markup(&format!("{{d|{}}}", age(a.at, ctx.now))),
        LW,
    )];
    if a.body.is_empty() {
        return out;
    }
    if matches!(a.kind, ActivityKind::Pushed { .. }) {
        out.push(markup(&format!("    {{d|{}}}", esc(&a.body))));
    } else {
        out.extend(super::markdown::render(&a.body, LW - 4).into_iter().map(|mut l| {
            l.insert(0, sp(4));
            l
        }));
    }
    out
}

/// The overview pane's rows: description, checks, conversation.
pub fn overview(d: &PrDetail, expanded: bool, ctx: &Ctx) -> Vec<Line> {
    let mut out = vec![vec![]];
    out.extend(super::markdown::render(&d.description, LW));
    if !d.checks.is_empty() {
        let mut checks: Vec<&CheckLine> = d.checks.iter().collect();
        // Problems first, so the folded view shows what matters.
        checks.sort_by_key(|c| match c.state {
            CheckState::Failed => 0,
            CheckState::Running | CheckState::Queued => 1,
            CheckState::Passed => 2,
        });
        let count = |st: CheckState| checks.iter().filter(|c| c.state == st).count();
        let mut summary = vec![format!("{} passed", count(CheckState::Passed))];
        if count(CheckState::Failed) > 0 {
            summary.push(format!("{} failing", count(CheckState::Failed)));
        }
        out.push(vec![]);
        out.push(lr(&markup("{d|checks}"), &markup(&format!("{{d|{}}}", summary.join(" · "))), LW));
        let shown = if expanded { checks.len() } else { FOLDED.min(checks.len()) };
        out.extend(checks[..shown].iter().map(|c| check_line(c)));
        let hidden = &checks[shown..];
        if !hidden.is_empty() {
            let all_passed = hidden.iter().all(|c| c.state == CheckState::Passed);
            let what =
                if all_passed { format!("+ {} more passed", hidden.len()) } else { format!("+ {} more", hidden.len()) };
            out.push(markup(&format!("{{d|  {what:<27}zo to expand}}")));
        }
    }
    if d.conversation_total > 0 || !d.conversation.is_empty() {
        out.push(vec![]);
        let total = d.conversation_total.max(d.conversation.len());
        out.push(lr(&markup("{d|conversation}"), &markup(&format!("{{d|{total}}}")), LW));
        out.extend(d.conversation.iter().flat_map(|a| activity(a, ctx)));
    }
    out
}

/// The diff pane's rows for one file: hunk headers dim, line numbers faint,
/// additions and removals in ok and err, a blank row between hunks.
pub fn diff_rows(f: &FileDiff) -> Vec<Line> {
    let mut out = vec![vec![]];
    for (i, h) in f.hunks.iter().enumerate() {
        if i > 0 {
            out.push(vec![]);
        }
        out.push(vec![s(&h.header, Tok::Dim)]);
        for l in &h.lines {
            let n = |x: Option<u32>| pad_start(&x.map_or(String::new(), |v| v.to_string()), 3);
            let (sign, fg) = match l.kind {
                Kind::Added => ("+", Tok::Ok),
                Kind::Removed => ("-", Tok::Err),
                Kind::Context => (" ", Tok::Fg),
            };
            out.push(vec![
                s(format!("{} {}  ", n(l.old), n(l.new)), Tok::Faint),
                s(sign, fg),
                s(format!(" {}", l.text), fg),
            ]);
        }
    }
    out
}

/// Indices of the files whose path contains `filter`, ignoring case.
pub fn matching(files: &[FileDiff], filter: &str) -> Vec<usize> {
    let f = filter.to_lowercase();
    files.iter().enumerate().filter(|(_, x)| x.path.to_lowercase().contains(&f)).map(|(i, _)| i).collect()
}

/// The `F` file list: every changed file with its size, filtered by what
/// has been typed, in the same surface panel as the `?` overlay.
pub fn picker(files: &[FileDiff], filter: &str, sel: usize, current: usize, h: usize) -> Vec<Line> {
    use super::help::PANEL_W;
    let pad = |l: &[Span]| {
        let mut v = vec![sp(3)];
        v.extend_from_slice(l);
        crate::span::with_bg(fit(&v, PANEL_W, true), Tok::Surf)
    };
    let title =
        if filter.is_empty() { markup("{B|files}") } else { markup(&format!("{{B|files}}{{a|  {}}}", esc(filter))) };
    let hint = markup("{d|type to filter · ↑↓ move · Enter open · Esc close}");
    let mut out = vec![pad(&[]), pad(&lr(&title, &hint, PANEL_W - 6)), pad(&[])];
    let shown = matching(files, filter);
    let room = h.saturating_sub(12).max(3);
    let start = sel.saturating_sub(room - 1);
    for (row, &i) in shown.iter().enumerate().skip(start).take(room) {
        let f = &files[i];
        let mark = if i == current { s("▌ ", Tok::Acc) } else { sp(2) };
        let size =
            markup(&format!("{{g|{:>6}}} {{r|{:<6}}}", format!("+{}", f.additions), format!("−{}", f.deletions)));
        let mut l = vec![mark];
        l.extend(size);
        l.push(sp(2));
        l.push(s(&f.path, Tok::Fg));
        let line = pad(&l);
        out.push(if row == sel {
            crate::span::with_bg(
                line.into_iter()
                    .map(|mut x| {
                        x.bg = None;
                        x
                    })
                    .collect(),
                Tok::Sel,
            )
        } else {
            line
        });
    }
    if shown.is_empty() {
        out.push(pad(&markup("{d|no file matches}")));
    }
    out.push(pad(&markup(&format!("{{d|{} of {} files}}", shown.len(), files.len()))));
    out.push(pad(&[]));
    out
}

/// Rows where each hunk's header sits, for `]c` and `[c`.
pub fn hunk_rows(f: &FileDiff) -> Vec<usize> {
    let mut rows = Vec::new();
    let mut r = 1;
    for (i, h) in f.hunks.iter().enumerate() {
        if i > 0 {
            r += 1;
        }
        rows.push(r);
        r += 1 + h.lines.len();
    }
    rows
}

fn heading(text: &str, focused: bool) -> String {
    if focused { format!("{{aB|{}}}", esc(text)) } else { format!("{{B|{}}}", esc(text)) }
}

pub fn detail_screen(o: &DetailScreen, ctx: &Ctx) -> Vec<Line> {
    let (w, pr, d) = (o.w, o.pr, o.detail);
    let v = verdict(pr, &ctx.caps(pr));
    let rw = w.saturating_sub(2 + LW + 3 + 1);
    let mut out = vec![
        lr(
            &[
                sp(2),
                s(o.section, Tok::Dim),
                s("  ›  ", Tok::Faint),
                s(format!("{}  {}  ", ctx.glyphs.forge(pr.id.forge), pr.id.repo), Tok::Dim),
                s(format!("#{}", pr.id.number), Tok::Fg).bold(),
            ],
            &[
                s(&o.position, Tok::Dim),
                sp(4),
                s(format!("updated {} ago", age(pr.updated_at, ctx.now)), Tok::Dim),
                sp(1),
            ],
            w,
        ),
        vec![],
        vec![sp(2), s(&pr.title, Tok::Fg).bold()],
    ];
    let who = if pr.author.is_agent { format!("◇ {}", pr.author.login) } else { pr.author.login.clone() };
    out.push(vec![
        sp(2),
        s(who, Tok::Fg),
        s(
            format!(
                "   {} → {}   +{} −{}   {} files   opened {} ago",
                pr.head,
                pr.base,
                pr.additions,
                pr.deletions,
                pr.files,
                age(pr.created_at, ctx.now)
            ),
            Tok::Dim,
        ),
    ]);
    out.push(vec![]);
    let sentence = match &pr.state {
        PrState::Open => verdict_long(pr, &v),
        PrState::Merged { method, at } => format!("Merged with {} {} ago.", method.word(), age(*at, ctx.now)),
        PrState::Closed => "Closed without merging.".into(),
    };
    out.push(vec![sp(2), label(&v, pr), sp(3), s(sentence, Tok::Fg)]);
    out.push(vec![]);
    let f = facts(pr, d, ctx);
    for (a, b) in [(&f[0], &f[2]), (&f[1], &f[3])] {
        // Two cells of the column are a gap, so a long left fact truncates
        // short of the right one instead of running into it.
        let mut l = vec![sp(2)];
        l.extend(fit(&markup(a), FACT_W - 2, true));
        l.push(sp(2));
        l.extend(markup(b));
        out.push(l);
    }
    out.push(vec![]);
    let file = o.files.and_then(|fs| fs.get(o.d.file));
    let (diff_head, right): (Line, Vec<Line>) = match (o.files, file) {
        (None, _) => (markup("{d|loading the diff…}"), vec![]),
        (Some(_), None) => (markup("{d|no changes}"), vec![]),
        (Some(fs), Some(f)) => (
            markup(&format!(
                "{{d|file {}/{}   }}{{g|+{}}}{{d| }}{{r|−{}}}",
                o.d.file + 1,
                fs.len(),
                f.additions,
                f.deletions
            )),
            diff_rows(f),
        ),
    };
    let path = file.map_or("diff", |f| f.path.as_str());
    let diff_title = markup(&heading(path, o.d.focus == Pane::Diff));
    let body = body_rows(o.h);
    let right: Vec<Line> = right.into_iter().skip(o.d.scroll[1]).collect();
    if o.d.zoom {
        let zw = w.saturating_sub(3);
        let mut l = vec![sp(2)];
        l.extend(fit(&lr(&diff_title, &diff_head, zw), zw, true));
        out.push(l);
        for i in 0..body {
            let mut l = vec![sp(2)];
            l.extend(fit(right.get(i).map_or(&[][..], |v| v), zw, true));
            out.push(l);
        }
    } else {
        let left: Vec<Line> = overview(d, o.d.expanded, ctx).into_iter().skip(o.d.scroll[0]).collect();
        let ov = markup(&heading("overview", o.d.focus == Pane::Overview));
        let mut l = vec![sp(2)];
        l.extend(fit(&lr(&ov, &markup("{d|description · checks · conversation}"), LW), LW, true));
        l.push(s(" │ ", Tok::Faint));
        l.extend(fit(&lr(&diff_title, &diff_head, rw), rw, true));
        out.push(l);
        for i in 0..body {
            let mut l = vec![sp(2)];
            l.extend(fit(left.get(i).map_or(&[][..], |v| v), LW, true));
            l.push(s(" │ ", Tok::Faint));
            l.extend(fit(right.get(i).map_or(&[][..], |v| v), rw, true));
            l.push(sp(1));
            out.push(l);
        }
    }
    out.push(status(w, &o.status));
    out.push(o.cmd.clone());
    out.into_iter().map(|l| fit(&l, w, false)).collect()
}
