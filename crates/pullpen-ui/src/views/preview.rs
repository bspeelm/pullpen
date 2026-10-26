//! The preview pane: "safe to merge?" for the selected row without opening
//! it (design §01). Laid out as tui.js `preview4821` lays out its one PR.

use pullpen_core::time::age;
use pullpen_core::{ActivityKind, Mergeable, Pr, PrDetail, PrState, Verdict, verdict};

use super::Ctx;
use super::words::verdict_short;
use crate::palette::Tok;
use crate::span::{Line, lr, markup, s, wrap};

/// The description is cut at a sentence end, so the preview never stops
/// mid-thought, and at this many lines, so the activity below stays visible.
const DESC_LINES: usize = 5;

pub fn verdict_word(v: &Verdict) -> Line {
    match v {
        Verdict::Ready => markup("{g|✓ ready}"),
        Verdict::Waiting(_) => markup("{y|○ waiting}"),
        Verdict::Blocked(_) => markup("{r|✗ blocked}"),
    }
}

/// The four facts behind the verdict, as (glyph markup, label, value, note).
pub fn facts(pr: &Pr) -> [(String, &'static str, String, String); 4] {
    let c = &pr.checks;
    let checks = if let Some(f) = c.failed.first() {
        ("{r|✗}".into(), format!("{} failing", f.name), f.detail.clone())
    } else if c.running > 0 || c.required_passed < c.required_total {
        ("{y|○}".into(), format!("{}/{} running", c.required_passed, c.required_total), String::new())
    } else if c.required_total == 0 {
        ("{g|✓}".into(), "none required".into(), String::new())
    } else {
        ("{g|✓}".into(), format!("{}/{} passed", c.required_passed, c.required_total), String::new())
    };
    let r = &pr.reviews;
    let reviews = if r.changes_requested {
        ("{r|✗}".into(), "changes requested".into(), String::new())
    } else if r.need == 0 {
        ("{g|✓}".into(), "none required".into(), String::new())
    } else {
        let note = if r.pending.is_empty() {
            String::new()
        } else {
            format!("{} pending", r.pending.iter().map(|p| format!("@{p}")).collect::<Vec<_>>().join(" "))
        };
        let g = if r.short() { "{y|○}" } else { "{g|✓}" };
        (g.into(), format!("{} of {}", r.have, r.need), note)
    };
    let conflicts = match pr.mergeable {
        Mergeable::Clean => ("{g|✓}".into(), "none".into(), "up to date".into()),
        Mergeable::Behind => ("{g|✓}".into(), "none".into(), format!("behind {}", pr.base)),
        Mergeable::Conflict => ("{r|≠}".into(), format!("with {}", pr.base), String::new()),
        Mergeable::Unknown => ("{y|○}".into(), "checking".into(), String::new()),
    };
    let auto = match &pr.auto_merge {
        Some(a) => ("{a|◆}".into(), "armed".into(), format!("{} · @{}", a.method.word(), a.armed_by)),
        None => ("{d|·}".into(), "off".into(), "A arms".into()),
    };
    [
        (checks.0, "checks", checks.1, checks.2),
        (reviews.0, "reviews", reviews.1, reviews.2),
        (conflicts.0, "conflicts", conflicts.1, conflicts.2),
        (auto.0, "auto-merge", auto.1, auto.2),
    ]
}

fn esc(t: &str) -> String {
    // Markup has no escape; a brace in user text would open a tag. A full-
    // width brace looks the same and cannot.
    t.replace('{', "｛").replace('}', "｝")
}

fn fact_line(f: &(String, &'static str, String, String)) -> Line {
    let note = if f.3.is_empty() { String::new() } else { format!("{{d|   {}}}", esc(&f.3)) };
    markup(&format!("{} {{d|{:<12}}}{{f|{}}}{note}", f.0, f.1, esc(&f.2)))
}

/// The first paragraph, cut after the last sentence that ends within
/// `DESC_LINES` wrapped lines. A first sentence longer than that is cut
/// at the line limit instead.
pub fn description(text: &str, w: usize) -> Vec<String> {
    let para = text.split("\n\n").next().unwrap_or("").replace('\n', " ");
    let all = wrap(para.trim(), w);
    if all.len() <= DESC_LINES {
        return all;
    }
    let mut best = None;
    for (i, _) in para.match_indices(". ") {
        let lines = wrap(&para[..=i], w);
        if lines.len() > DESC_LINES {
            break;
        }
        best = Some(lines);
    }
    best.unwrap_or_else(|| {
        let mut l: Vec<String> = all.into_iter().take(DESC_LINES).collect();
        if let Some(last) = l.last_mut() {
            last.push('…');
        }
        l
    })
}

fn who(name: &str, agent: bool) -> String {
    if agent { format!("◇ {}", esc(name)) } else { esc(name) }
}

fn activity_line(a: &pullpen_core::Activity, w: usize, ctx: &Ctx) -> Line {
    let (g, what) = match &a.kind {
        ActivityKind::Approved => ("{g|✓}", " approved".to_string()),
        ActivityKind::ChangesRequested => ("{r|✗}", " requested changes".to_string()),
        ActivityKind::ArmedAutoMerge => ("{a|◆}", " armed auto-merge".to_string()),
        ActivityKind::DisarmedAutoMerge => ("{d|·}", " disarmed auto-merge".to_string()),
        ActivityKind::Pushed { commits: 1 } => ("{d|·}", " pushed 1 commit".to_string()),
        ActivityKind::Pushed { commits } => ("{d|·}", format!(" pushed {commits} commits")),
        ActivityKind::ReviewRequested { .. } => ("{y|○}", " review requested".to_string()),
        ActivityKind::Commented => ("{d|·}", " commented".to_string()),
    };
    lr(
        &markup(&format!("{g} {{f|{}}}{{d|{what}}}", who(&a.who, a.is_agent))),
        &markup(&format!("{{d|{}}}", age(a.at, ctx.now))),
        w,
    )
}

pub fn preview(pr: &Pr, d: &PrDetail, w: usize, ctx: &Ctx) -> Vec<Line> {
    let v = verdict(pr, &ctx.caps(pr));
    let tag = ctx.glyphs.forge(pr.id.forge);
    let mut out = vec![
        vec![s(&pr.title, Tok::Fg).bold()],
        markup(&format!(
            "{{d|{tag}  {} #{}   }}{{f|{}}}{{d|   {}}}",
            esc(&pr.id.repo),
            pr.id.number,
            who(&pr.author.login, pr.author.is_agent),
            age(pr.updated_at, ctx.now)
        )),
        vec![],
    ];
    let vl = match &pr.state {
        PrState::Merged { at, method } => {
            markup(&format!("{{d|● merged  {} · {} ago}}", method.word(), age(*at, ctx.now)))
        }
        PrState::Closed => markup("{d|● closed without merging}"),
        PrState::Open => {
            let mut vl = verdict_word(&v);
            vl.extend(markup(&format!("{{d|  {}}}", esc(&verdict_short(pr, &v)))));
            vl
        }
    };
    out.push(vl);
    out.push(vec![]);
    out.extend(facts(pr).iter().map(fact_line));
    out.push(vec![]);
    out.push(markup(&format!("{{d|{} → {}}}", esc(&pr.head), esc(&pr.base))));
    out.push(markup(&format!("{{d|+{} −{} · {} files}}", pr.additions, pr.deletions, pr.files)));
    if !d.description.is_empty() {
        out.push(vec![]);
        out.extend(description(&d.description, w).into_iter().map(|t| vec![s(t, Tok::Fg)]));
    }
    if !d.activity.is_empty() {
        out.push(vec![]);
        out.push(markup("{d|activity}"));
        out.extend(d.activity.iter().map(|a| activity_line(a, w, ctx)));
    }
    out
}
