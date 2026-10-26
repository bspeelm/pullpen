//! Application state and how it becomes a frame. Keys are handled in
//! `input`, actions and their results in `act`; the binary owns the
//! terminal and runs the effects this returns.

pub mod act;
mod armed;
pub use armed::Watch;
pub mod cmdline;
pub mod command;
mod detail;
pub mod input;
mod mouse;
pub use mouse::Mouse;

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use pullpen_core::diff::FileDiff;
use pullpen_core::time::Timestamp;
use pullpen_core::{Capabilities, Forge, Job, MergeMethod, Pr, PrDetail, PrId, Readiness, Section, readiness};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use self::act::{Batch, Prompt, Running};
use self::cmdline::{Completion, Editor};
use crate::glyphs::SPINNER;
use crate::paint::{Theme, paint};
use crate::palette::Tok;
use crate::span::{Line, fit, lr, markup, overlay, s, sp, with_bg};
use crate::views::Ctx;
use crate::views::chrome::{Mode, Status, Tab, status};
use crate::views::help;
use crate::views::list::{ListScreen, PREVIEW_W, RowOpts, body_rows, list_screen, row, state_glyph};
use crate::views::preview::preview;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PreviewMode {
    /// Shown at `PREVIEW_MIN_W` columns and wider (design §03).
    #[default]
    Auto,
    On,
    Off,
}

pub const PREVIEW_MIN_W: usize = 110;

#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub text: String,
    /// Enter was pressed: non-matches are hidden, not just faded.
    pub applied: bool,
}

pub struct SectionState {
    pub section: Section,
    pub prs: Vec<Pr>,
    /// What the forge reported, which can exceed the rows loaded.
    pub total: usize,
    /// Index into the visible rows.
    pub sel: usize,
    pub offset: usize,
    pub marks: Vec<PrId>,
    pub filter: Option<Filter>,
}

impl SectionState {
    pub fn new(section: Section, prs: Vec<Pr>, total: usize) -> Self {
        SectionState { section, prs, total, sel: 0, offset: 0, marks: Vec::new(), filter: None }
    }

    fn matches(&self, p: &Pr) -> bool {
        self.filter.as_ref().is_none_or(|f| p.title.to_lowercase().contains(&f.text.to_lowercase()))
    }

    /// The rows on screen: all of them, or only matches once a filter is
    /// applied.
    pub fn visible(&self) -> Vec<&Pr> {
        let hide = self.filter.as_ref().is_some_and(|f| f.applied);
        self.prs.iter().filter(|p| !hide || self.matches(p)).collect()
    }

    pub fn selected(&self) -> Option<&Pr> {
        self.visible().get(self.sel).copied()
    }

    /// Positions of filter matches among the visible rows.
    pub fn match_rows(&self) -> Vec<usize> {
        self.visible().iter().enumerate().filter(|(_, p)| self.matches(p)).map(|(i, _)| i).collect()
    }
}

/// How merges choose a method when the command does not say (PLAN.md §9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DefaultMethod {
    /// The repo's own default; arming asks when the repo allows several.
    #[default]
    Repo,
    Fixed(MergeMethod),
}

pub struct UiConfig {
    pub preview: PreviewMode,
    pub short_names_below: usize,
    pub default_method: DefaultMethod,
    pub delete_branch: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            preview: PreviewMode::Auto,
            short_names_below: 100,
            default_method: DefaultMethod::Repo,
            delete_branch: true,
        }
    }
}

/// What has the keyboard.
#[derive(Clone, Debug, Default)]
pub enum Input {
    #[default]
    Normal,
    Command(Editor),
    Filter(Editor),
    Confirm(Prompt),
    Help(String),
    Messages,
    /// A command failed; the line keeps it so the next key can fix it.
    CommandError {
        text: String,
        suggestion: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct Message {
    pub line: Line,
    /// The mode the status line shows while the message stands: a failed
    /// command stays in COMMAND so it can be fixed (design §07).
    pub mode: Mode,
}

/// Work the binary does on the app's behalf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Run(Job),
    /// Suspend, open $EDITOR, and come back with `App::comment_text`.
    Edit {
        targets: Vec<PrId>,
    },
    Open(String),
    Refresh {
        all: bool,
    },
    FetchDetail(PrId),
    FetchDiff(PrId),
    /// Ring the bell and send an OSC 9 notification, as configured.
    Notify(String),
    Redraw,
    SaveHistory,
}

pub const LOG_MAX: usize = 200;

pub struct App {
    pub sections: Vec<SectionState>,
    pub active: usize,
    pub details: HashMap<PrId, PrDetail>,
    pub diffs: HashMap<PrId, Vec<FileDiff>>,
    /// Fetches already asked for, as (PR, is the diff).
    pub requested: HashSet<(PrId, bool)>,
    pub failed: HashSet<(PrId, bool)>,
    pub detail: Option<crate::views::detail::Detail>,
    pub watches: HashMap<PrId, armed::Watch>,
    pub notify: Vec<String>,
    /// When the user last left pullpen, for "merged since".
    pub last_visit: Option<Timestamp>,
    /// Seconds east of UTC, for showing clock times.
    pub tz_offset: i64,
    pub last_click: Option<(usize, Instant)>,
    pub caps: HashMap<String, Capabilities>,
    pub forges: Vec<Forge>,
    pub theme: Theme,
    pub ui: UiConfig,
    pub me: String,
    pub input: Input,
    pub message: Option<Message>,
    pub log: VecDeque<Line>,
    /// Oldest first.
    pub history: Vec<String>,
    pub pending: Option<(char, Instant)>,
    pub jobs: Vec<Running>,
    pub batches: Vec<Batch>,
    pub next_job: u64,
    /// Advances while work is running; the spinner frame is `tick % 4`.
    pub tick: usize,
    pub sync: String,
    pub err: Option<(Forge, String)>,
    pub quit: bool,
    /// The last frame's size, for half-page moves and the preview toggle.
    pub height: usize,
    pub last_width: usize,
    pub now: Timestamp,
}

impl App {
    pub fn new(sections: Vec<SectionState>, theme: Theme, now: Timestamp) -> Self {
        let mut app = App {
            sections,
            active: 0,
            details: HashMap::new(),
            diffs: HashMap::new(),
            requested: HashSet::new(),
            failed: HashSet::new(),
            detail: None,
            watches: HashMap::new(),
            notify: Vec::new(),
            last_visit: None,
            tz_offset: 0,
            last_click: None,
            caps: HashMap::new(),
            forges: vec![Forge::GitHub, Forge::GitLab],
            theme,
            ui: UiConfig::default(),
            me: String::new(),
            input: Input::Normal,
            message: None,
            log: VecDeque::new(),
            history: Vec::new(),
            pending: None,
            jobs: Vec::new(),
            batches: Vec::new(),
            next_job: 1,
            tick: 0,
            sync: String::new(),
            err: None,
            quit: false,
            height: 24,
            last_width: 120,
            now,
        };
        app.arrange();
        app
    }

    pub fn section(&self) -> &SectionState {
        &self.sections[self.active]
    }

    pub fn section_mut(&mut self) -> &mut SectionState {
        &mut self.sections[self.active]
    }

    pub fn ctx(&self) -> Ctx<'_> {
        Ctx { now: self.now, glyphs: self.theme.glyphs, caps: &self.caps }
    }

    pub fn caps_of(&self, p: &Pr) -> Capabilities {
        self.caps.get(&format!("{}/{}", p.id.host, p.id.repo)).cloned().unwrap_or_default()
    }

    pub fn readiness(&self, p: &Pr) -> Readiness {
        readiness(p, &self.caps_of(p))
    }

    pub fn find(&self, id: &PrId) -> Option<&Pr> {
        self.sections.iter().flat_map(|s| &s.prs).find(|p| p.id == *id)
    }

    /// Armed and failing PRs across every section that counts, each PR once
    /// however many sections it appears in.
    pub fn counts(&self) -> (usize, usize) {
        let mut seen = std::collections::HashSet::new();
        let (mut armed, mut failing) = (0, 0);
        for p in self.sections.iter().filter(|s| s.section.counts).flat_map(|s| &s.prs) {
            if seen.insert(&p.id) {
                armed += usize::from(p.armed());
                failing += usize::from(self.readiness(p) == Readiness::Failing);
            }
        }
        (armed, failing)
    }

    pub fn spinner(&self) -> char {
        SPINNER[self.tick % SPINNER.len()]
    }

    pub fn busy(&self, id: &PrId) -> bool {
        self.jobs.iter().any(|j| j.job.target == *id)
    }

    pub fn preview_shown(&self, w: usize) -> bool {
        match self.ui.preview {
            PreviewMode::Auto => w >= PREVIEW_MIN_W,
            PreviewMode::On => true,
            PreviewMode::Off => false,
        }
    }

    pub fn narrow(&self, w: usize) -> bool {
        w < self.ui.short_names_below
    }

    /// Keeps the selection inside the visible window for an `h`-row screen.
    pub fn scroll_into_view(&mut self, h: usize) {
        let rows = body_rows(h).max(1);
        let s = self.section_mut();
        if s.sel < s.offset {
            s.offset = s.sel;
        } else if s.sel >= s.offset + rows {
            s.offset = s.sel + 1 - rows;
        }
    }

    pub fn mode(&self) -> Mode {
        match (&self.input, &self.message) {
            (Input::Command(_) | Input::CommandError { .. }, _) => Mode::Command,
            (Input::Filter(_), _) => Mode::Filter,
            (Input::Confirm(_), _) => Mode::Confirm,
            (_, Some(m)) => m.mode,
            _ => Mode::Normal,
        }
    }

    pub fn status(&self, w: usize) -> Status<'_> {
        let sec = self.section();
        let narrow = self.narrow(w);
        let pos = match (&self.input, &sec.filter) {
            (Input::Filter(_), Some(_)) => {
                let m = sec.match_rows();
                let k = m.iter().position(|i| *i == sec.sel).map_or(0, |k| k + 1);
                format!("{k}/{} matches", m.len())
            }
            _ if sec.prs.is_empty() => String::new(),
            (_, Some(f)) if f.applied => format!("{}/{}", sec.sel + 1, sec.visible().len()),
            _ => format!("{}/{}", sec.sel + 1, sec.total),
        };
        let extra =
            if sec.marks.is_empty() { vec![] } else { vec![s(format!("{} marked", sec.marks.len()), Tok::Acc)] };
        let (armed, failing) = self.counts();
        // The detail view names the PR where the list names the section, and
        // the focused pane where the list gives a position (design §04).
        let (section, pos) = match &self.detail {
            Some(d) => {
                let pane = if d.focus == crate::views::detail::Pane::Diff { "diff" } else { "overview" };
                (format!("#{}", d.id.number), pane.to_string())
            }
            None => (if narrow { sec.section.short.clone() } else { sec.section.name.clone() }, pos),
        };
        Status {
            mode: self.mode(),
            section,
            pos,
            extra,
            armed,
            failing,
            narrow,
            err: self.err.clone(),
            sync: self.sync.clone(),
            forges: &self.forges,
        }
    }

    /// The last row: the line being edited, a prompt, a result, or progress.
    pub fn cmd_line(&self, w: usize) -> Line {
        match &self.input {
            Input::Command(e) => {
                // An open menu is the resolution; the right side stays clear.
                let right = match e.walk_label() {
                    Some(h) => markup(&format!("{{d|{h}}}  ")),
                    None if e.menu.is_some() => vec![],
                    None => match self.live_target(&e.text) {
                        Some(t) => markup(&format!("{{d|→ {t}}}  ")),
                        None => vec![],
                    },
                };
                lr(&e.line(':'), &right, w)
            }
            Input::Filter(e) => lr(&e.line('/'), &markup("{d|n N jump   Enter hide non-matches   Esc clear}"), w),
            Input::Confirm(p) => self.prompt_line(p),
            _ => match &self.message {
                Some(m) => m.line.clone(),
                None => self.progress_line(),
            },
        }
    }

    fn progress_line(&self) -> Line {
        let Some(r) = self.jobs.iter().find(|j| j.batch.is_none()) else { return vec![] };
        let host = &r.job.target.host;
        markup(&format!(
            "{{a|{}}} {{f|{} #{} on {host}…}}",
            self.spinner(),
            act::verb(&r.job.action),
            r.job.target.number
        ))
    }

    fn live_target(&self, text: &str) -> Option<String> {
        let p = command::parse(text);
        let t = p.target?;
        let rows: Vec<Pr> = self.section().visible().into_iter().cloned().collect();
        Some(match command::resolve(&t, &rows) {
            command::Resolved::One(id) => format!("{} #{}", id.repo, id.number),
            command::Resolved::None => "no such PR here".into(),
            command::Resolved::Ambiguous(repos) => format!("in {}: say which", repos.join(", ")),
        })
    }

    /// The wildmenu: (rows, width), drawn upward over the list.
    pub fn menu(&self) -> Option<(Vec<Line>, usize)> {
        let Input::Command(e) = &self.input else { return None };
        let m = e.menu.as_ref()?;
        let ctx = self.ctx();
        match m.kind {
            Completion::Command | Completion::Section => {
                let lines = m
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, name)| {
                        let what = command::COMMANDS.iter().find(|c| c.name == name).map_or("", |c| c.what);
                        let mut n = s(name, Tok::Fg);
                        if i == m.sel {
                            n = n.bold();
                        }
                        let gap = sp(14usize.saturating_sub(name.chars().count()));
                        let l = fit(&[sp(1), n, gap, s(what, Tok::Dim)], 56, true);
                        if i == m.sel { with_bg(l, Tok::Sel) } else { l }
                    })
                    .collect();
                Some((lines, 56))
            }
            Completion::Target => {
                let rows = self.section().visible();
                let lines = m
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, item)| {
                        let n: u64 = item.trim_start_matches(['#', '!']).parse().ok()?;
                        let p = rows.iter().find(|p| p.id.number == n)?;
                        let sel = i == m.sel;
                        let mut num = s(format!("{:<7}", format!("#{n}")), Tok::Fg);
                        if sel {
                            num = num.bold();
                        }
                        let mut l = vec![sp(1), state_glyph(ctx.readiness(p)), sp(1), num];
                        l.extend(fit(&[s(&p.title, Tok::Fg)], 40, true));
                        l.push(sp(2));
                        l.push(s(&p.id.repo, Tok::Dim));
                        let l = fit(&l, 84, true);
                        Some(if sel { with_bg(l, Tok::Sel) } else { l })
                    })
                    .collect();
                Some((lines, 84))
            }
        }
    }

    pub fn row_opts(&self, i: usize, p: &Pr) -> RowOpts<'_> {
        let s = self.section();
        let hl = s.filter.as_ref().map(|f| f.text.as_str()).filter(|t| !t.is_empty());
        RowOpts {
            sel: i == s.sel,
            mark: s.marks.contains(&p.id),
            hl,
            dim_all: hl.is_some() && !s.matches(p),
            spin: self.busy(&p.id).then(|| self.spinner()),
        }
    }

    pub fn lines(&self, w: usize, h: usize) -> Vec<Line> {
        let ctx = self.ctx();
        let sec = self.section();
        let tabs: Vec<Tab> = self
            .sections
            .iter()
            .map(|x| Tab { name: x.section.name.clone(), short: x.section.short.clone(), count: x.total })
            .collect();
        let rows: Vec<Pr> = sec.visible().into_iter().cloned().collect();
        let pv = (self.preview_shown(w) && !rows.is_empty()).then(|| {
            let p = &rows[sec.sel.min(rows.len() - 1)];
            let empty = PrDetail::default();
            preview(p, self.details.get(&p.id).unwrap_or(&empty), PREVIEW_W, &ctx)
        });
        let opts: Vec<RowOpts> = rows.iter().enumerate().map(|(i, p)| self.row_opts(i, p)).collect();
        let screen = ListScreen {
            w,
            h,
            tabs: &tabs,
            active: self.active,
            short: self.narrow(w),
            right: &sec.section.query,
            prs: &rows,
            opts: &opts,
            offset: sec.offset,
            preview: pv,
            status: self.status(w),
            cmd: self.cmd_line(w),
        };
        let mut lines = match self.detail_lines(w, h).or_else(|| self.armed_lines(w, h)) {
            Some(l) => l,
            None => list_screen(&screen, &ctx),
        };
        if let Some((menu, mw)) = self.menu() {
            let bottom = lines.len().saturating_sub(2);
            let top = bottom.saturating_sub(menu.len());
            for (i, m) in menu.iter().enumerate() {
                let r = &mut lines[top + i];
                *r = overlay(r, 0, &with_bg(fit(m, mw.min(w), true), Tok::Surf));
            }
        }
        match &self.input {
            Input::Help(f) => {
                help::fade(&mut lines, h.saturating_sub(2));
                help::place(&mut lines, &help::panel(self.sections.len(), f), w);
            }
            Input::Messages => {
                help::fade(&mut lines, h.saturating_sub(2));
                help::place(&mut lines, &act::messages_panel(&self.log, h), w);
            }
            _ => {}
        }
        lines
    }

    fn detail_lines(&self, w: usize, h: usize) -> Option<Vec<Line>> {
        let d = self.detail.as_ref()?;
        let pr = self.find(&d.id)?;
        let sec = self.section();
        let at = sec.prs.iter().position(|p| p.id == d.id).map_or(0, |i| i + 1);
        let empty = PrDetail::default();
        let screen = crate::views::detail::DetailScreen {
            w,
            h,
            pr,
            detail: self.details.get(&d.id).unwrap_or(&empty),
            files: self.diffs.get(&d.id).map(Vec::as_slice),
            d,
            section: &sec.section.name,
            position: format!("{at} of {}", sec.total),
            status: self.status(w),
            cmd: self.cmd_line(w),
        };
        Some(crate::views::detail::detail_screen(&screen, &self.ctx()))
    }

    /// The design's row strips (tui.js `strip`): the first `k` rows, the
    /// wildmenu over them, then status and command line. `pos` overrides the
    /// status position where the design leaves it blank.
    pub fn strip(&self, k: usize, w: usize, pos: Option<&str>) -> Vec<Line> {
        let ctx = self.ctx();
        let rows: Vec<Pr> = self.section().visible().into_iter().take(k).cloned().collect();
        let mut out: Vec<Line> =
            rows.iter().enumerate().map(|(i, p)| fit(&row(p, w - 1, &self.row_opts(i, p), &ctx), w, false)).collect();
        if let Some((menu, mw)) = self.menu() {
            let n = menu.len();
            for (i, m) in menu.iter().enumerate() {
                let ix = out.len() - n + i;
                out[ix] = overlay(&out[ix], 0, &with_bg(fit(m, mw, true), Tok::Surf));
            }
        }
        let mut st = self.status(w);
        if let Some(p) = pos {
            st.pos = p.into();
        }
        out.push(status(w, &st));
        out.push(fit(&self.cmd_line(w), w, true));
        out
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let (w, h) = (area.width as usize, area.height as usize);
        self.height = h;
        self.last_width = w;
        self.scroll_into_view(h);
        let lines = self.lines(w, h);
        paint(&lines, buf, area, &self.theme);
    }

    /// New rows for a section from a sync. The cursor stays on the same PR
    /// if it is still there, else at the same index (design §13); marks on
    /// PRs that left are dropped.
    pub fn synced(&mut self, i: usize, mut prs: Vec<Pr>, total: usize) {
        self.track(i, &mut prs);
        let total = if self.armed_view(i) { self.armed_count(&prs) } else { total };
        // A PR that changed has a stale detail and diff; a fetch that failed
        // gets another try.
        for p in &prs {
            if self.find(&p.id).is_some_and(|old| old.updated_at != p.updated_at) {
                self.details.remove(&p.id);
                self.diffs.remove(&p.id);
                self.requested.retain(|(id, _)| *id != p.id);
            }
        }
        for f in std::mem::take(&mut self.failed) {
            self.requested.remove(&f);
        }
        let Some(s) = self.sections.get_mut(i) else { return };
        let keep = s.selected().map(|p| p.id.clone());
        // Rows with an action in flight keep their optimistic state until the
        // forge answers the action itself.
        for p in &mut prs {
            if let Some(old) = s.prs.iter().find(|o| o.id == p.id)
                && self.jobs.iter().any(|j| j.job.target == p.id)
            {
                p.auto_merge = old.auto_merge.clone();
            }
        }
        s.prs = prs;
        s.total = total;
        s.marks.retain(|m| s.prs.iter().any(|p| p.id == *m));
        let vis: Vec<PrId> = s.visible().iter().map(|p| p.id.clone()).collect();
        s.sel = keep.and_then(|id| vis.iter().position(|v| *v == id)).unwrap_or(s.sel).min(vis.len().saturating_sub(1));
    }

    /// A forge failing in the background: the status line says so until it
    /// recovers (design §10).
    pub fn forge_down(&mut self, f: Forge, msg: String) {
        if self.err.as_ref().is_none_or(|(_, m)| *m != msg) {
            self.log.push_back(markup(&format!("{{r|E  {}}}", act::esc(&msg))));
        }
        self.err = Some((f, msg));
    }

    pub fn forge_up(&mut self, f: Forge) {
        if self.err.as_ref().is_some_and(|(e, _)| *e == f) {
            self.err = None;
        }
    }

    pub fn say(&mut self, line: Line, mode: Mode) {
        self.log.push_back(line.clone());
        while self.log.len() > LOG_MAX {
            self.log.pop_front();
        }
        self.message = Some(Message { line, mode });
    }

    pub fn error(&mut self, text: &str, mode: Mode) {
        self.say(markup(&format!("{{r|E  {}}}", act::esc(text))), mode);
    }
}

#[cfg(test)]
mod tests;
