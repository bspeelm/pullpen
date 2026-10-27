use pullpen_fixtures as fx;

use super::*;
use crate::glyphs::Glyphs;
use crate::palette::{Palette, Tier};

fn app() -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW)
}

/// The defaults plus the agent-opened rows as a seventh section, counted as
/// a configured section is.
const AGENT: usize = 6;

fn with_agents() -> App {
    let sections = fx::sections()
        .into_iter()
        .chain([fx::agent_section()])
        .map(|s| SectionState::new(s.section, s.prs, s.total))
        .collect();
    App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW)
}

#[test]
fn status_counts_skip_the_catch_all_and_count_each_pr_once() {
    // #2290 is the one failure in the counted sections; Open holds the rest
    // (#814, #4802, #143, #371), and it counts nothing. #4821 is in Armed
    // and Open and counts once.
    assert_eq!(app().counts(), (7, 1));
    assert_eq!(with_agents().counts(), (7, 3), "an Agent section added back counts #143 and #371 too");
}

#[test]
fn preview_hides_below_110_columns_unless_forced() {
    let mut a = app();
    assert!(!a.preview_shown(109));
    assert!(a.preview_shown(110));
    a.ui.preview = PreviewMode::On;
    assert!(a.preview_shown(80));
}

#[test]
fn the_selection_stays_on_screen() {
    let mut a = with_agents();
    a.active = AGENT;
    a.section_mut().sel = 22;
    a.scroll_into_view(24);
    assert_eq!(a.section().offset, 22 + 1 - body_rows(24));
    a.section_mut().sel = 0;
    a.scroll_into_view(24);
    assert_eq!(a.section().offset, 0);
}

#[test]
fn a_sync_keeps_the_cursor_on_the_same_pr_or_the_same_index() {
    let mut a = with_agents();
    a.active = AGENT;
    a.section_mut().sel = 2;
    let mut prs = a.section().prs.clone();
    prs.remove(0);
    a.synced(AGENT, prs.clone(), 22);
    assert_eq!(a.section().selected().unwrap().id.number, 2290, "followed #2290 up one row");
    let gone = prs.remove(1);
    assert_eq!(gone.id.number, 2290);
    a.synced(AGENT, prs, 21);
    assert_eq!(a.section().sel, 1, "#2290 left, so the same index");
    assert_eq!(a.section().total, 21);
}

#[test]
fn a_forge_error_clears_when_that_forge_recovers() {
    let mut a = app();
    a.forge_down(Forge::GitLab, "gitlab.com 502".into());
    a.forge_up(Forge::GitHub);
    assert!(a.err.is_some(), "a different forge recovering changes nothing");
    a.forge_up(Forge::GitLab);
    assert!(a.err.is_none());
}
