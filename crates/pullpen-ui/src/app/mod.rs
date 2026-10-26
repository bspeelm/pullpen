//! Application state and how it becomes a frame. Input handling lives
//! beside it; the binary owns the terminal and the forge.

use std::collections::HashMap;

use pullpen_core::time::Timestamp;
use pullpen_core::{Capabilities, Forge, Pr, PrDetail, PrId, Readiness, Section, readiness};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::paint::{Theme, paint};
use crate::span::Line;
use crate::views::Ctx;
use crate::views::chrome::{Mode, Status, Tab};
use crate::views::list::{ListScreen, PREVIEW_W, body_rows, list_screen};
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

pub struct SectionState {
    pub section: Section,
    pub prs: Vec<Pr>,
    /// What the forge reported, which can exceed the rows loaded.
    pub total: usize,
    pub sel: usize,
    pub offset: usize,
}

pub struct UiConfig {
    pub preview: PreviewMode,
    pub short_names_below: usize,
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            preview: PreviewMode::Auto,
            short_names_below: 100,
        }
    }
}

pub struct App {
    pub sections: Vec<SectionState>,
    pub active: usize,
    pub details: HashMap<PrId, PrDetail>,
    pub caps: HashMap<String, Capabilities>,
    pub forges: Vec<Forge>,
    pub theme: Theme,
    pub ui: UiConfig,
    pub mode: Mode,
    pub cmd: Line,
    pub sync: String,
    pub err: Option<(Forge, String)>,
    pub quit: bool,
}

impl App {
    pub fn section(&self) -> &SectionState {
        &self.sections[self.active]
    }

    fn ctx(&self, now: Timestamp) -> Ctx<'_> {
        Ctx {
            now,
            glyphs: self.theme.glyphs,
            caps: &self.caps,
        }
    }

    /// Armed and failing PRs across every section that counts, each PR once
    /// however many sections it appears in.
    pub fn counts(&self) -> (usize, usize) {
        let mut seen = std::collections::HashSet::new();
        let (mut armed, mut failing) = (0, 0);
        for s in self.sections.iter().filter(|s| s.section.counts) {
            for p in &s.prs {
                if !seen.insert(&p.id) {
                    continue;
                }
                armed += usize::from(p.armed());
                let caps = self
                    .caps
                    .get(&format!("{}/{}", p.id.host, p.id.repo))
                    .cloned()
                    .unwrap_or_default();
                failing += usize::from(readiness(p, &caps) == Readiness::Failing);
            }
        }
        (armed, failing)
    }

    pub fn preview_shown(&self, w: usize) -> bool {
        match self.ui.preview {
            PreviewMode::Auto => w >= PREVIEW_MIN_W,
            PreviewMode::On => true,
            PreviewMode::Off => false,
        }
    }

    /// Keeps the selection inside the visible window for an `h`-row screen.
    pub fn scroll_into_view(&mut self, h: usize) {
        let rows = body_rows(h).max(1);
        let s = &mut self.sections[self.active];
        if s.sel < s.offset {
            s.offset = s.sel;
        } else if s.sel >= s.offset + rows {
            s.offset = s.sel + 1 - rows;
        }
    }

    pub fn lines(&self, w: usize, h: usize, now: Timestamp) -> Vec<Line> {
        let ctx = self.ctx(now);
        let s = self.section();
        let tabs: Vec<Tab> = self
            .sections
            .iter()
            .map(|s| Tab {
                name: s.section.name.clone(),
                short: s.section.short.clone(),
                count: s.total,
            })
            .collect();
        let pv = (self.preview_shown(w) && !s.prs.is_empty()).then(|| {
            let p = &s.prs[s.sel.min(s.prs.len() - 1)];
            let empty = PrDetail::default();
            preview(
                p,
                self.details.get(&p.id).unwrap_or(&empty),
                PREVIEW_W,
                &ctx,
            )
        });
        let (armed, failing) = self.counts();
        let narrow = w < self.ui.short_names_below;
        let pos = if s.prs.is_empty() {
            String::new()
        } else {
            format!("{}/{}", s.sel + 1, s.total)
        };
        let screen = ListScreen {
            w,
            h,
            tabs: &tabs,
            active: self.active,
            short: narrow,
            right: &s.section.query,
            prs: &s.prs,
            sel: s.sel,
            offset: s.offset,
            marks: &[],
            hl: None,
            preview: pv,
            status: Status {
                mode: self.mode,
                section: if narrow {
                    &s.section.short
                } else {
                    &s.section.name
                },
                pos,
                extra: vec![],
                armed,
                failing,
                narrow,
                err: self.err.clone(),
                sync: self.sync.clone(),
                forges: &self.forges,
            },
            cmd: self.cmd.clone(),
        };
        list_screen(&screen, &ctx)
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, now: Timestamp) {
        let (w, h) = (area.width as usize, area.height as usize);
        self.scroll_into_view(h);
        let lines = self.lines(w, h, now);
        paint(&lines, buf, area, &self.theme);
    }
}
