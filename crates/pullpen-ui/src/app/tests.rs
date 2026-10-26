use pullpen_fixtures as fx;

use super::*;
use crate::glyphs::Glyphs;
use crate::palette::{Palette, Tier};

fn app() -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW)
}

#[test]
fn status_counts_skip_the_catch_all_and_count_each_pr_once() {
    // #4821 is in Agent-opened and Auto-merge armed; All holds the human
    // failures (#814, #4802), which are not counted.
    assert_eq!(app().counts(), (7, 3));
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
    let mut a = app();
    a.active = 2;
    a.section_mut().sel = 22;
    a.scroll_into_view(24);
    assert_eq!(a.section().offset, 22 + 1 - body_rows(24));
    a.section_mut().sel = 0;
    a.scroll_into_view(24);
    assert_eq!(a.section().offset, 0);
}

#[test]
fn a_sync_keeps_the_cursor_on_the_same_pr_or_the_same_index() {
    let mut a = app();
    a.active = 2;
    a.section_mut().sel = 2;
    let mut prs = a.section().prs.clone();
    prs.remove(0);
    a.synced(2, prs.clone(), 22);
    assert_eq!(a.section().selected().unwrap().id.number, 2290, "followed #2290 up one row");
    let gone = prs.remove(1);
    assert_eq!(gone.id.number, 2290);
    a.synced(2, prs, 21);
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
