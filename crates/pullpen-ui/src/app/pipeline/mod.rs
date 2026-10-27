//! The pipeline and job screens' keys, data and refresh (ADR-020): a
//! subscreen of the PR, asked for again every few seconds while anything
//! in it is still running, and not at all once it has finished.

use std::time::{Duration, Instant};

use pullpen_core::time::{Timestamp, duration};
use pullpen_core::{CheckState, Forge, JobRun, PrId, Run};

use super::cmdline::Editor;
use super::{App, Effect, Input};
use crate::keymap::Key;
use crate::span::{Line, markup};
use crate::views::log::{self, Log};
use crate::views::pipeline::{
    JobScreen, JobView, Pipe, PipelineScreen, job_rows, job_screen, log_top, pipeline_screen, steps,
};

/// How often a watched pipeline is asked for while it runs; `sync.watch`.
pub const WATCH: Duration = Duration::from_secs(5);

/// When each of the watched things was last asked for, and whether an
/// answer is still on its way.
#[derive(Clone, Debug, Default)]
pub struct Poll {
    pub runs_asked: Option<Instant>,
    pub runs_waiting: bool,
    /// When the runs last arrived, for "refreshed 3s ago".
    pub runs_at: Option<Timestamp>,
    pub log_asked: Option<Instant>,
    pub log_waiting: bool,
    /// The log was fetched after its job finished, so it is complete.
    pub log_final: bool,
}

impl App {
    /// `w`: the selected PR's pipeline, or the open detail view's.
    pub fn open_pipe(&mut self) {
        let id = match &self.detail {
            Some(d) => Some(d.id.clone()),
            None => self.section().selected().map(|p| p.id.clone()),
        };
        if let Some(id) = id {
            self.watch_pr(id);
        }
    }

    pub fn watch_pr(&mut self, id: PrId) {
        self.pipe = Some(Pipe::new(id));
        self.poll = Poll::default();
    }

    pub fn runs_of(&self, id: &PrId) -> Option<&[Run]> {
        self.runs.get(id).map(Vec::as_slice)
    }

    /// The open job, and the run it is in.
    pub fn open_job(&self) -> Option<(&Run, &JobRun, &JobView)> {
        let p = self.pipe.as_ref()?;
        let v = p.job.as_ref()?;
        let run = self.runs_of(&p.id)?.iter().find(|r| r.which == v.which)?;
        let job = run.jobs.iter().find(|j| j.name == v.name)?;
        Some((run, job, v))
    }

    /// Why a job's log cannot be shown, or `None` when it can be asked for.
    pub fn no_log(&self, id: &PrId, job: &JobRun) -> Option<String> {
        if !job.log || job.id.is_empty() {
            return Some("this check has no log here · o opens it".into());
        }
        if id.forge == Forge::GitHub && matches!(job.state, CheckState::Running | CheckState::Queued) {
            return Some("GitHub publishes a job's log when the job finishes · o opens it live".into());
        }
        if job.state == CheckState::Queued {
            return Some("the job has not started".into());
        }
        None
    }

    /// What the watched screens need asked for now. Called every frame, so
    /// it is cheap when there is nothing to do.
    pub fn watch(&mut self, now: Instant) -> Vec<Effect> {
        let Some(id) = self.pipe.as_ref().map(|p| p.id.clone()) else { return vec![] };
        let every = self.ui.watch;
        let due = |asked: Option<Instant>| asked.is_none_or(|a| now.duration_since(a) >= every);
        let mut out = Vec::new();
        let running = self.runs_of(&id).is_none_or(|rs| rs.is_empty() || rs.iter().any(|r| !r.done()));
        if !self.poll.runs_waiting && (self.poll.runs_asked.is_none() || running && due(self.poll.runs_asked)) {
            self.poll.runs_asked = Some(now);
            self.poll.runs_waiting = true;
            out.push(Effect::FetchRuns(id.clone()));
        }
        let job = self.open_job().map(|(_, j, _)| j.clone());
        if let Some(j) = job
            && self.no_log(&id, &j).is_none()
            && !self.poll.log_waiting
            && !self.poll.log_final
        {
            let done = matches!(j.state, CheckState::Passed | CheckState::Failed);
            let have = self.logs.contains_key(&(id.clone(), j.id.clone()));
            if !have || done || due(self.poll.log_asked) {
                self.poll.log_asked = Some(now);
                self.poll.log_waiting = true;
                self.poll.log_final = done;
                out.push(Effect::FetchLog(id, j.id));
            }
        }
        out
    }

    pub fn runs_loaded(&mut self, id: PrId, runs: Vec<Run>) {
        self.poll.runs_waiting = false;
        // A run finishing changes the checks the detail view shows.
        let finished = |rs: &[Run]| rs.iter().filter(|r| r.done()).count();
        if self.runs.get(&id).is_some_and(|old| finished(old) != finished(&runs)) {
            self.details.remove(&id);
            self.requested.retain(|(i, _)| *i != id);
        }
        // A job open since before it finished needs its whole log once.
        if let Some((_, j, _)) = self.open_job()
            && j.state == CheckState::Running
            && runs.iter().flat_map(|r| &r.jobs).any(|n| n.id == j.id && n.state != CheckState::Running)
        {
            self.poll.log_final = false;
            self.poll.log_asked = None;
        }
        let first = self.poll.runs_at.is_none();
        self.poll.runs_at = Some(self.now);
        self.runs.insert(id, runs);
        if let Some(p) = self.pipe.as_mut() {
            let runs = self.runs.get(&p.id).map_or(&[][..], Vec::as_slice);
            let rows = p.rows(runs);
            // The first answer puts the cursor where the news is: the first
            // failed job, else the first still running.
            let at = |st: CheckState| rows.iter().position(|(r, j)| runs[*r].jobs[*j].state == st);
            if first {
                p.sel = at(CheckState::Failed).or_else(|| at(CheckState::Running)).unwrap_or(0);
            }
            p.sel = p.sel.min(rows.len().saturating_sub(1));
        }
    }

    pub fn log_loaded(&mut self, id: PrId, job: String, raw: &str) {
        self.poll.log_waiting = false;
        self.logs.insert((id.clone(), job), log::parse(raw, id.forge == Forge::GitLab));
    }

    /// A runs or log fetch failed: say so, and try again at the next tick.
    pub fn watch_failed(&mut self, log: bool, why: &str) {
        if log {
            self.poll.log_waiting = false;
            self.poll.log_final = false;
        } else {
            self.poll.runs_waiting = false;
        }
        self.error(why, crate::views::chrome::Mode::Normal);
    }

    pub fn current_log(&self) -> Option<&Log> {
        let (_, j, _) = self.open_job()?;
        self.logs.get(&(self.pipe.as_ref()?.id.clone(), j.id.clone()))
    }

    pub(super) fn pipe_lines(&self, w: usize, h: usize) -> Option<Vec<Line>> {
        let p = self.pipe.as_ref()?;
        // The active section's copy: a PR merged since it was listed
        // elsewhere is merged here.
        let pr = self.section().prs.iter().find(|x| x.id == p.id).or_else(|| self.find(&p.id))?;
        let ctx = self.ctx();
        let section = &self.section().section.name;
        let refreshed = match self.poll.runs_at {
            Some(t) => format!("refreshed {} ago", duration(t, self.now)),
            None => String::new(),
        };
        if let Some((run, job, view)) = self.open_job() {
            let screen = JobScreen {
                w,
                h,
                pr,
                run,
                job,
                log: self.current_log(),
                note: self.no_log(&p.id, job),
                view,
                section,
                refreshed,
                status: self.status(w),
                cmd: self.cmd_line(w),
            };
            return Some(job_screen(&screen, &ctx));
        }
        let screen = PipelineScreen {
            w,
            h,
            pr,
            runs: self.runs_of(&p.id),
            pipe: p,
            section,
            refreshed,
            status: self.status(w),
            cmd: self.cmd_line(w),
        };
        Some(pipeline_screen(&screen, &ctx))
    }

    /// The status line's position: the job among the jobs, or the log row
    /// at the bottom of the screen.
    pub(super) fn pipe_pos(&self) -> String {
        let Some(p) = &self.pipe else { return String::new() };
        if let Some((_, _, v)) = self.open_job() {
            let len = self.current_log().map_or(0, |l| l.lines.len());
            let bottom = (log_top(v, len, job_rows(self.height)) + job_rows(self.height)).min(len);
            return format!("log {bottom}/{len}");
        }
        let n = p.rows(self.runs_of(&p.id).unwrap_or_default()).len();
        if n == 0 { String::new() } else { format!("job {}/{n}", p.sel + 1) }
    }

    pub(super) fn pipe_hint(&self) -> Line {
        let Some(p) = &self.pipe else { return vec![] };
        let text = if p.job.is_some() {
            format!(
                "Enter jump to step   G follow   / search   e next error   o open on {}   q back to the pipeline",
                p.id.host
            )
        } else {
            format!(
                "Enter open job   zo zc show / fold a run   o open on {}   r refresh   q back to #{}",
                p.id.host, p.id.number
            )
        };
        markup(&format!("{{d|{text}}}"))
    }

    /// Keys while a pipeline or job screen is open; `None` lets the key
    /// through to the command line and help.
    pub(super) fn pipe_key(&mut self, key: Key, now: Instant) -> Option<Vec<Effect>> {
        let pipe = self.pipe.as_ref()?;
        if matches!(key, Key::Char(':' | '?')) {
            return None;
        }
        let pending = self.pending.take().map(|(c, _)| c);
        Some(if pipe.job.is_some() { self.job_key(key, pending, now) } else { self.runs_key(key, pending, now) })
    }

    fn runs_key(&mut self, key: Key, pending: Option<char>, now: Instant) -> Vec<Effect> {
        let Some(mut p) = self.pipe.clone() else { return vec![] };
        let runs = self.runs.get(&p.id).cloned().unwrap_or_default();
        let rows = p.rows(&runs);
        let max = rows.len().saturating_sub(1);
        let mut out = vec![];
        match (pending, key) {
            (Some('g'), Key::Char('g')) => p.sel = 0,
            (Some('z'), Key::Char(c @ ('o' | 'c'))) => {
                // zc folds the run the cursor is in; zo shows the first run
                // that is folded.
                let at = rows.get(p.sel).map_or(0, |(r, _)| *r);
                let target = if c == 'c' {
                    runs.get(at)
                } else {
                    runs.iter().enumerate().find(|(i, r)| !p.shown(*i, r)).map(|(_, r)| r)
                };
                if let Some(w) = target.map(|r| r.which) {
                    match p.toggled.iter().position(|t| *t == w) {
                        Some(k) => drop(p.toggled.remove(k)),
                        None => p.toggled.push(w),
                    }
                }
                p.sel = p.sel.min(p.rows(&runs).len().saturating_sub(1));
            }
            (_, Key::Char(c @ ('g' | 'z'))) => self.pending = Some((c, now)),
            (_, Key::Char('j') | Key::Down) => p.sel = (p.sel + 1).min(max),
            (_, Key::Char('k') | Key::Up) => p.sel = p.sel.saturating_sub(1),
            (_, Key::Char('G')) => p.sel = max,
            (_, Key::Char('q') | Key::Esc) => {
                self.pipe = None;
                return vec![];
            }
            (_, Key::Char('r')) => self.poll.runs_asked = None,
            (_, Key::Enter) => {
                if let Some((r, j)) = rows.get(p.sel) {
                    p.job = Some(JobView::new(runs[*r].which, &runs[*r].jobs[*j]));
                    self.poll.log_final = false;
                    self.poll.log_asked = None;
                }
            }
            (_, Key::Char('o')) => {
                let job = rows.get(p.sel).map(|(r, j)| runs[*r].jobs[*j].url.clone());
                let url = job.filter(|u| !u.is_empty()).or_else(|| runs.first().map(|r| r.url.clone()));
                out = url.filter(|u| !u.is_empty()).map(Effect::Open).into_iter().collect();
            }
            _ => {}
        }
        self.pipe = Some(p);
        out
    }

    fn job_key(&mut self, key: Key, pending: Option<char>, now: Instant) -> Vec<Effect> {
        let Some(mut v) = self.pipe.as_ref().and_then(|p| p.job.clone()) else { return vec![] };
        let rows = job_rows(self.height);
        let len = self.current_log().map_or(0, |l| l.lines.len());
        let errors: Vec<usize> = self
            .current_log()
            .map_or(vec![], |l| l.lines.iter().enumerate().filter(|(_, x)| x.error).map(|(i, _)| i).collect());
        let starts: Vec<Option<usize>> = self
            .open_job()
            .map_or(vec![], |(_, j, _)| steps(j, self.current_log(), self.now).iter().map(|s| s.line).collect());
        let url = self.open_job().map(|(_, j, _)| j.url.clone());
        let top = log_top(&v, len, rows);
        let last = len.saturating_sub(rows);
        // Scrolling moves the step marker to the step the top row is in.
        let at = |v: &mut JobView, line: usize| {
            v.follow = false;
            v.scroll = line.min(last);
            v.step = starts.iter().rposition(|s| s.is_some_and(|l| l <= v.scroll)).unwrap_or(0);
        };
        let mut out = vec![];
        match (pending, key) {
            (Some('g'), Key::Char('g')) if v.steps => v.step = 0,
            (Some('g'), Key::Char('g')) => at(&mut v, 0),
            (_, Key::Char('g')) => self.pending = Some(('g', now)),
            (_, Key::Char('h')) => v.steps = true,
            (_, Key::Char('l')) => v.steps = false,
            (_, Key::Char('j') | Key::Down) if v.steps => v.step = (v.step + 1).min(starts.len().saturating_sub(1)),
            (_, Key::Char('k') | Key::Up) if v.steps => v.step = v.step.saturating_sub(1),
            (_, Key::Enter) if v.steps => match starts.get(v.step) {
                Some(Some(line)) => {
                    let chosen = v.step;
                    at(&mut v, *line);
                    // A short log cannot scroll that far; the marker stays
                    // on the step chosen.
                    v.step = chosen;
                    v.steps = false;
                }
                _ => self.refused("that step has nothing in the log yet"),
            },
            (_, Key::Char('j') | Key::Down) => at(&mut v, top + 1),
            (_, Key::Char('k') | Key::Up) => at(&mut v, top.saturating_sub(1)),
            (_, Key::Ctrl('d')) => at(&mut v, top + rows / 2),
            (_, Key::Ctrl('u')) => at(&mut v, top.saturating_sub(rows / 2)),
            (_, Key::Char('G')) => v.follow = true,
            (_, Key::Char('e')) => match errors.iter().find(|e| **e > top).or(errors.first()) {
                Some(e) => at(&mut v, *e),
                None => self.refused("no error lines in this log"),
            },
            (_, Key::Char(c @ ('n' | 'N'))) => {
                if let Some(line) = self.next_hit(top, c == 'n') {
                    at(&mut v, line);
                }
            }
            (_, Key::Char('/')) => self.input = Input::Search(Editor::with(v.search.as_deref().unwrap_or_default())),
            (_, Key::Char('q') | Key::Esc) => {
                if let Some(p) = self.pipe.as_mut() {
                    p.job = None;
                }
                return vec![];
            }
            (_, Key::Char('r')) => {
                self.poll.runs_asked = None;
                self.poll.log_asked = None;
            }
            (_, Key::Char('o')) => out = url.filter(|u| !u.is_empty()).map(Effect::Open).into_iter().collect(),
            _ => {}
        }
        if let Some(p) = self.pipe.as_mut() {
            p.job = Some(v);
        }
        out
    }

    /// The next log row after `from` (or before, going back) that has the
    /// search text in it, wrapping.
    fn next_hit(&self, from: usize, fwd: bool) -> Option<usize> {
        let want = self.open_job()?.2.search.clone()?.to_lowercase();
        let lines = &self.current_log()?.lines;
        let hit = |i: &usize| lines[*i].text.to_lowercase().contains(&want);
        let n = lines.len();
        if fwd {
            (from + 1..n).chain(0..=from.min(n.saturating_sub(1))).find(hit)
        } else {
            (0..from).rev().chain((from..n).rev()).find(hit)
        }
    }

    /// `/` in the job screen: each keystroke jumps to the first match from
    /// where the log stands.
    pub(super) fn search_key(&mut self, mut e: Editor, key: Key) -> Vec<Effect> {
        let set = |a: &mut App, text: Option<String>| {
            if let Some(v) = a.pipe.as_mut().and_then(|p| p.job.as_mut()) {
                v.search = text;
            }
        };
        match key {
            Key::Esc => {
                set(self, None);
                return vec![];
            }
            Key::Enter => return vec![],
            k => e.edit(k),
        }
        set(self, (!e.text.is_empty()).then(|| e.text.clone()));
        let rows = job_rows(self.height);
        let len = self.current_log().map_or(0, |l| l.lines.len());
        let top = self.open_job().map_or(0, |(_, _, v)| log_top(v, len, rows));
        if let Some(line) = self.next_hit(top.saturating_sub(1), true)
            && let Some(v) = self.pipe.as_mut().and_then(|p| p.job.as_mut())
        {
            v.follow = false;
            v.scroll = line.min(len.saturating_sub(rows));
        }
        self.input = Input::Search(e);
        vec![]
    }
}

#[cfg(test)]
mod tests;
