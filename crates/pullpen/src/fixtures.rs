//! `--fixtures`: the design's data, offline (PLAN.md §8).

use std::collections::HashMap;

use pullpen_core::Forge;
use pullpen_fixtures as fx;
use pullpen_ui::app::{App, SectionState, UiConfig};
use pullpen_ui::paint::Theme;
use pullpen_ui::views::chrome::Mode;

pub fn app(theme: Theme) -> App {
    let sections: Vec<SectionState> = fx::sections()
        .into_iter()
        .map(|s| SectionState {
            section: s.section,
            prs: s.prs,
            total: s.total,
            sel: 0,
            offset: 0,
        })
        .collect();
    let mut details = HashMap::new();
    for s in &sections {
        for p in &s.prs {
            details
                .entry(p.id.clone())
                .or_insert_with(|| fx::detail(p.id.number));
        }
    }
    App {
        sections,
        active: 2,
        details,
        caps: HashMap::new(),
        forges: vec![Forge::GitHub, Forge::GitLab],
        theme,
        ui: UiConfig::default(),
        mode: Mode::Normal,
        cmd: vec![],
        sync: "fixtures".into(),
        err: None,
        quit: false,
    }
}
