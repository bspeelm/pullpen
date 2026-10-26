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
