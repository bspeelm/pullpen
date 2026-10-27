//! A PR's pipeline and one of its jobs (tui.js `pipelineScreen`,
//! `jobScreen`): subscreens of the PR, which stays named on top (ADR-020).

use pullpen_core::time::{Timestamp, age, duration};
use pullpen_core::{CheckState, Forge, JobRun, Pr, PrId, PrState, Run, Which};

use super::Ctx;
use super::chrome::{Status, status};
use super::detail::esc;
use super::log::Log;
use crate::palette::Tok;
use crate::span::{Line, fit, lr, markup, s, sp, with_bg};

/// The steps pane's width in the job screen.
const LW: usize = 40;
/// Rows above the body: the PR's four, a gap, the job, a gap, the headings.
const JOB_TOP: usize = 8;

/// The pipeline screen's state; `job` is set while a job is open over it.
#[derive(Clone, Debug)]
pub struct Pipe {
    pub id: PrId,
    /// The selected job, among the jobs of the runs shown.
    pub sel: usize,
    /// Runs whose fold `zo`/`zc` changed. The first run starts shown and
    /// the rest folded.
    pub toggled: Vec<Which>,
    pub job: Option<JobView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobView {
    pub which: Which,
    /// The job's name, which outlives a refresh where an index might not.
    pub name: String,
    /// First log row shown, while not following.
    pub scroll: usize,
    pub follow: bool,
    /// Focus on the steps rather than the log, and the step under the cursor.
    pub steps: bool,
    pub step: usize,
    pub search: Option<String>,
}

impl Pipe {
    pub fn new(id: PrId) -> Self {
        Pipe { id, sel: 0, toggled: vec![], job: None }
    }

    pub fn shown(&self, i: usize, run: &Run) -> bool {
        (i == 0) != self.toggled.contains(&run.which)
    }

    /// Every selectable job, as (run, job) indices, in screen order.
    pub fn rows(&self, runs: &[Run]) -> Vec<(usize, usize)> {
        runs.iter()
            .enumerate()
            .filter(|(i, r)| self.shown(*i, r))
            .flat_map(|(i, r)| (0..r.jobs.len()).map(move |j| (i, j)))
            .collect()
    }
}

impl JobView {
    pub fn new(which: Which, job: &JobRun) -> Self {
        JobView { which, name: job.name.clone(), scroll: 0, follow: true, steps: false, step: 0, search: None }
    }
}

pub struct PipelineScreen<'a> {
    pub w: usize,
    pub h: usize,
    pub pr: &'a Pr,
    /// `None` until the forge first answers.
    pub runs: Option<&'a [Run]>,
    pub pipe: &'a Pipe,
    pub section: &'a str,
    pub refreshed: String,
    pub status: Status<'a>,
    pub cmd: Line,
}

pub struct JobScreen<'a> {
    pub w: usize,
    pub h: usize,
    pub pr: &'a Pr,
    pub run: &'a Run,
    pub job: &'a JobRun,
    pub log: Option<&'a Log>,
    /// Why there is no log to show, when there is none.
    pub note: Option<String>,
    pub view: &'a JobView,
    pub section: &'a str,
    pub refreshed: String,
    pub status: Status<'a>,
    pub cmd: Line,
}

fn glyph(st: CheckState) -> &'static str {
    match st {
        CheckState::Passed => "{g|✓}",
        CheckState::Failed => "{r|✗}",
        CheckState::Running => "{y|○}",
        CheckState::Queued => "{d|·}",
    }
}

fn took(from: Option<Timestamp>, to: Option<Timestamp>, now: Timestamp) -> String {
    match (from, to) {
        (Some(a), Some(b)) => duration(a, b),
        (Some(a), None) => duration(a, now),
        _ => String::new(),
    }
}

/// The PR, named as the detail view names it, then the crumb this screen is.
fn head(pr: &Pr, section: &str, crumb: &str, refreshed: &str, w: usize, ctx: &Ctx) -> Vec<Line> {
    let who = if pr.author.is_agent { format!("◇ {}", pr.author.login) } else { pr.author.login.clone() };
    let when = match &pr.state {
        PrState::Merged { at, method } => format!("merged {} ago · {}", age(*at, ctx.now), method.word()),
        _ => format!("opened {} ago", age(pr.created_at, ctx.now)),
    };
    vec![
        lr(
            &[
                sp(2),
                s(section, Tok::Dim),
                s("  ›  ", Tok::Faint),
                s(format!("{}  {}  ", ctx.glyphs.forge(pr.id.forge), pr.id.repo), Tok::Dim),
                s(format!("#{}", pr.id.number), Tok::Fg).bold(),
                s("  ›  ", Tok::Faint),
                s(crumb, Tok::Fg),
            ],
            &[s(refreshed, Tok::Dim), sp(1)],
            w,
        ),
        vec![],
        vec![sp(2), s(&pr.title, Tok::Fg).bold()],
        vec![sp(2), s(who, Tok::Fg), s(format!("   {} → {}   {when}", pr.head, pr.base), Tok::Dim)],
        vec![],
    ]
}

fn which(r: &Run, pr: &Pr) -> (&'static str, String) {
    match r.which {
        Which::AfterMerge => ("after merge", format!("{} @ {}", pr.base, r.sha)),
        Which::BeforeMerge => ("before merge", r.sha.clone()),
    }
}

fn run_head(r: &Run, pr: &Pr, w: usize, ctx: &Ctx) -> Line {
    let (name, at) = which(r, pr);
    let passed = r.jobs.iter().filter(|j| j.state == CheckState::Passed).count();
    let time = match (r.started, r.done()) {
        (None, _) => "queued".to_string(),
        (Some(_), false) => format!("running {}", took(r.started, None, ctx.now)),
        (Some(_), true) => took(r.started, r.finished, ctx.now),
    };
    let left = markup(&format!("  {} {{B|{name}}}{{d|   {}   {}}}", glyph(r.state), esc(&at), esc(&r.event)));
    lr(&left, &[s(format!("{passed} of {} passed   {time}", r.jobs.len()), Tok::Dim), sp(1)], w)
}

/// What a job is doing, in the column between its stage and its time.
fn progress(j: &JobRun) -> String {
    let at = |st: CheckState| j.steps.iter().position(|x| x.state == st);
    match j.state {
        CheckState::Passed => "{d|passed}".into(),
        CheckState::Queued => "{d|queued}".into(),
        CheckState::Failed => match at(CheckState::Failed) {
            Some(i) => format!("{{r|failed}}{{d|  at {}}}", esc(&j.steps[i].name)),
            None => "{r|failed}".into(),
        },
        CheckState::Running => match at(CheckState::Running) {
            Some(i) => format!("{{y|running}}{{d|  step {} of {} · {}}}", i + 1, j.steps.len(), esc(&j.steps[i].name)),
            None => "{y|running}".into(),
        },
    }
}

fn job_row(j: &JobRun, sel: bool, w: usize, ctx: &Ctx) -> Line {
    let mut name = s(&j.name, Tok::Fg);
    if sel {
        name = name.bold();
    }
    let mut l = vec![if sel { s("▌ ", Tok::Acc) } else { sp(2) }];
    l.extend(markup(glyph(j.state)));
    l.push(sp(2));
    l.extend(fit(&[name], 30, true));
    l.extend(fit(&[s(&j.group, Tok::Dim)], 12, true));
    l.extend(fit(&markup(&progress(j)), w.saturating_sub(2 + 1 + 2 + 30 + 12 + 9), true));
    let t = if j.state == CheckState::Queued { String::new() } else { took(j.started, j.finished, ctx.now) };
    l.push(s(format!("{t:>8}"), Tok::Dim));
    l.push(sp(1));
    let l = fit(&l, w, false);
    if sel { with_bg(l, Tok::Sel) } else { l }
}

pub fn pipeline_screen(o: &PipelineScreen, ctx: &Ctx) -> Vec<Line> {
    let (w, pr) = (o.w, o.pr);
    let mut out = head(pr, o.section, "pipeline", &o.refreshed, w, ctx);
    let mut body: Vec<Line> = Vec::new();
    // The body row of the selected job, to keep it on screen.
    let mut at = 0;
    match o.runs {
        None => body.push(markup("  {d|asking for the pipeline…}")),
        Some([]) => body.push(markup("  {d|no pipeline has run for this PR}")),
        Some(runs) => {
            let mut k = 0;
            for (i, r) in runs.iter().enumerate() {
                if i > 0 {
                    body.push(vec![]);
                }
                body.push(run_head(r, pr, w, ctx));
                if !o.pipe.shown(i, r) {
                    body.push(markup(&format!("      {{d|zo to show its {} jobs}}", r.jobs.len())));
                    continue;
                }
                for j in &r.jobs {
                    if k == o.pipe.sel {
                        at = body.len();
                    }
                    body.push(job_row(j, k == o.pipe.sel, w, ctx));
                    k += 1;
                }
            }
        }
    }
    let room = o.h.saturating_sub(out.len() + 2);
    let skip = (at + 1).saturating_sub(room);
    out.extend(body.into_iter().skip(skip).take(room));
    while out.len() < o.h.saturating_sub(2) {
        out.push(vec![]);
    }
    out.push(status(w, &o.status));
    out.push(o.cmd.clone());
    out.into_iter().map(|l| fit(&l, w, false)).collect()
}

/// A step as the job screen lists it, from GitHub's steps or, on GitLab,
/// the log's sections.
pub struct StepRow {
    pub name: String,
    pub state: CheckState,
    pub took: String,
    /// The log row it starts at, when the log shows it.
    pub line: Option<usize>,
}

pub fn steps(job: &JobRun, log: Option<&Log>, now: Timestamp) -> Vec<StepRow> {
    let first_at = |t: Option<Timestamp>| {
        let (t, log) = (t?, log?);
        log.lines.iter().position(|l| l.at.is_some_and(|a| a >= t))
    };
    if !job.steps.is_empty() {
        return job
            .steps
            .iter()
            .map(|x| StepRow {
                name: x.name.clone(),
                state: x.state,
                took: if x.state == CheckState::Queued { String::new() } else { took(x.started, x.finished, now) },
                line: first_at(x.started),
            })
            .collect();
    }
    let Some(log) = log else { return vec![] };
    let n = log.sections.len();
    log.sections
        .iter()
        .enumerate()
        .map(|(i, x)| StepRow {
            name: x.name.clone(),
            // A section has no outcome of its own: the last one carries the
            // job's, and those before it finished.
            state: match (x.finished, i + 1 == n) {
                (_, true) if job.state != CheckState::Running => job.state,
                (None, _) => CheckState::Running,
                _ => CheckState::Passed,
            },
            took: took(Some(x.started), x.finished.or(job.finished), now),
            line: Some(x.line),
        })
        .collect()
}

/// The log's first row shown: the tail while following.
pub fn log_top(view: &JobView, len: usize, rows: usize) -> usize {
    if view.follow { len.saturating_sub(rows) } else { view.scroll.min(len.saturating_sub(1)) }
}

pub fn job_rows(h: usize) -> usize {
    h.saturating_sub(JOB_TOP + 2)
}

pub fn job_screen(o: &JobScreen, ctx: &Ctx) -> Vec<Line> {
    let (w, pr, j, v) = (o.w, o.pr, o.job, o.view);
    let mut out = head(pr, o.section, &j.name, &o.refreshed, w, ctx);
    let (run_name, at) = which(o.run, pr);
    let kind = if pr.id.forge == Forge::GitLab { "stage" } else { "workflow" };
    let state = match j.state {
        CheckState::Passed => format!("passed {}", took(j.started, j.finished, ctx.now)),
        CheckState::Failed => format!("failed {}", took(j.started, j.finished, ctx.now)),
        CheckState::Running => format!("running {}", took(j.started, None, ctx.now)),
        CheckState::Queued => "queued".into(),
    };
    out.push(lr(
        &markup(&format!(
            "  {} {{B|{}}}{{d|   {} {kind} · {run_name} · {}}}",
            glyph(j.state),
            esc(&j.name),
            esc(&j.group),
            esc(&at)
        )),
        &[s(state.trim_end(), Tok::Dim), sp(1)],
        w,
    ));
    out.push(vec![]);
    let rw = w.saturating_sub(2 + LW + 3 + 1);
    let rows = job_rows(o.h);
    let st = steps(j, o.log, ctx.now);
    let lines = o.log.map_or(&[][..], |l| l.lines.as_slice());
    let top = log_top(v, lines.len(), rows);
    let heading = |t: &str, focused: bool| if focused { format!("{{aB|{t}}}") } else { format!("{{B|{t}}}") };
    let tail = match o.log {
        None => String::new(),
        Some(l) if l.trimmed => format!("{{d|last {} lines}}", lines.len()),
        Some(_) => format!("{{d|{} lines}}", lines.len()),
    };
    let tail = if v.follow && o.log.is_some() { format!("{tail}{{d|   }}{{a|following ↓}}") } else { tail };
    let mut l = vec![sp(2)];
    l.extend(fit(&lr(&markup(&heading("steps", v.steps)), &markup(&format!("{{d|{}}}", st.len())), LW), LW, true));
    l.push(s(" │ ", Tok::Faint));
    l.extend(fit(&lr(&markup(&heading("log", !v.steps)), &markup(&tail), rw), rw, true));
    out.push(l);
    // The step the cursor is on when the steps have focus; otherwise the
    // one the log's last shown row is in, or the one running.
    let bottom = (top + rows).min(lines.len()).saturating_sub(1);
    let current = if v.steps {
        Some(v.step)
    } else {
        st.iter()
            .rposition(|x| x.line.is_some_and(|l| l <= bottom))
            .or_else(|| st.iter().position(|x| x.state == CheckState::Running))
    };
    let left: Vec<Line> = st
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let mut l = vec![if current == Some(i) { s("▌ ", Tok::Acc) } else { sp(2) }];
            l.extend(lr(&markup(&format!("{} {}", glyph(x.state), esc(&x.name))), &[s(&x.took, Tok::Dim)], LW - 2));
            l
        })
        .collect();
    let right: Vec<Line> = match (&o.note, o.log) {
        (Some(n), _) => vec![markup(&format!("{{d|{}}}", esc(n)))],
        (None, None) => vec![markup("{d|asking for the log…}")],
        (None, Some(_)) => lines[top..].iter().take(rows).map(|x| x.spans.clone()).collect(),
    };
    for i in 0..rows {
        let mut l = vec![sp(2)];
        l.extend(fit(left.get(i).map_or(&[][..], |v| v), LW, true));
        l.push(s(" │ ", Tok::Faint));
        l.extend(fit(right.get(i).map_or(&[][..], |v| v), rw, true));
        l.push(sp(1));
        out.push(l);
    }
    out.push(status(w, &o.status));
    out.push(o.cmd.clone());
    out.into_iter().map(|l| fit(&l, w, false)).collect()
}

#[cfg(test)]
mod tests;
