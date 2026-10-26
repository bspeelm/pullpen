use std::time::Instant;

use pullpen_fixtures as fx;

use super::*;
use crate::app::SectionState;
use crate::glyphs::Glyphs;
use crate::paint::Theme;
use crate::palette::{Palette, Tier};

fn app() -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    let mut a = App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW);
    a.active = 2;
    a.height = 40;
    a
}

#[test]
fn clicking_a_tab_switches_and_a_row_selects_and_twice_opens() {
    let mut a = app();
    let t = Instant::now();
    // "  Mine 6    Needs my review 4 …": column 12 is inside the second tab.
    a.mouse(Mouse::Click { x: 12, y: 0 }, t);
    assert_eq!(a.active, 1);
    a.active = 2;
    a.mouse(Mouse::Click { x: 30, y: 5 }, t);
    assert_eq!((a.section().sel, a.detail.is_none()), (2, true));
    a.mouse(Mouse::Click { x: 30, y: 5 }, t + Duration::from_millis(100));
    assert!(a.detail.is_some(), "a double click opens the row");
}

#[test]
fn the_wheel_moves_the_selection() {
    let mut a = app();
    a.mouse(Mouse::Scroll { x: 0, down: true }, Instant::now());
    assert_eq!(a.section().sel, 3);
}
