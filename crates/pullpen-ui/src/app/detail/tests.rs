use std::time::Instant;

use pullpen_fixtures as fx;

use crate::app::{App, Effect, SectionState};
use crate::glyphs::Glyphs;
use crate::keymap::Key;
use crate::paint::Theme;
use crate::palette::{Palette, Tier};
use crate::views::detail::Pane;

fn opened() -> App {
    // The agent-opened rows, as a section of their own (not a default).
    let sections = fx::sections()
        .into_iter()
        .chain([fx::agent_section()])
        .map(|s| SectionState::new(s.section, s.prs, s.total))
        .collect();
    let mut a = App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW);
    a.active = 6;
    a.height = 40;
    let id = a.section().prs[0].id.clone();
    a.details.insert(id.clone(), fx::detail(4821));
    a.diffs.insert(id, fx::diff(4821));
    a.key(Key::Enter, Instant::now());
    a
}

fn keys(a: &mut App, s: &str) {
    for c in s.chars() {
        a.key(Key::Char(c), Instant::now());
    }
}

fn d(a: &App) -> &crate::views::detail::Detail {
    a.detail.as_ref().unwrap()
}

#[test]
fn files_step_forward_and_back_and_stop_at_the_ends() {
    let mut a = opened();
    keys(&mut a, "[f");
    assert_eq!(d(&a).file, 0, "nothing before the first file");
    for _ in 0..20 {
        keys(&mut a, "]f");
    }
    assert_eq!(d(&a).file, 13, "the fourteenth file is the last");
    keys(&mut a, "[f[f");
    assert_eq!(d(&a).file, 11);
}

#[test]
fn hunks_step_within_a_file_then_on_to_the_next() {
    let mut a = opened();
    keys(&mut a, "]f");
    keys(&mut a, "]c");
    assert_eq!((d(&a).file, d(&a).scroll[1], d(&a).focus), (1, 1, Pane::Diff), "first hunk header");
    keys(&mut a, "]c");
    assert_eq!(d(&a).scroll[1], 21, "second hunk, after 18 lines and a blank");
    keys(&mut a, "[c");
    assert_eq!(d(&a).scroll[1], 1);
    keys(&mut a, "]c]c");
    assert_eq!((d(&a).file, d(&a).scroll[1]), (2, 1), "past the last hunk, the next file's first");
    keys(&mut a, "]c");
    assert_eq!((d(&a).file, d(&a).scroll[1]), (2, 6), "then that file's second");
}

#[test]
fn zo_and_zc_unfold_and_fold_the_checks() {
    let mut a = opened();
    keys(&mut a, "zo");
    assert!(d(&a).expanded);
    let text: String = a.lines(120, 40).iter().flatten().map(|s| s.text.clone()).collect();
    assert!(!text.contains("more passed"), "unfolded, every check is listed");
    keys(&mut a, "zc");
    assert!(!d(&a).expanded);
}

#[test]
fn d_zooms_the_diff_and_q_goes_back_with_the_cursor_kept() {
    let mut a = opened();
    keys(&mut a, "d");
    assert!(d(&a).zoom && d(&a).focus == Pane::Diff);
    keys(&mut a, "q");
    assert!(a.detail.is_none());
    assert_eq!(a.section().sel, 0);
}

#[test]
fn merging_from_the_detail_view_goes_back_to_the_list_when_done() {
    let mut a = opened();
    a.key(Key::Char('q'), Instant::now());
    a.key(Key::Char('j'), Instant::now());
    a.key(Key::Enter, Instant::now());
    let fx = [Key::Char('M'), Key::Char('y')].iter().flat_map(|k| a.key(*k, Instant::now())).collect::<Vec<_>>();
    for e in fx {
        if let Effect::Run(job) = e {
            a.job_done(pullpen_core::JobResult { job, outcome: pullpen_core::Outcome::Done, note: None });
        }
    }
    assert!(a.detail.is_none(), "#812 merged; its detail view closed");
}

#[test]
fn opening_asks_for_the_detail_and_diff_once() {
    let mut a = opened();
    let id = a.section().prs[1].id.clone();
    a.key(Key::Char('q'), Instant::now());
    a.key(Key::Char('j'), Instant::now());
    a.key(Key::Enter, Instant::now());
    let first = a.wants();
    assert!(first.contains(&Effect::FetchDetail(id.clone())) && first.contains(&Effect::FetchDiff(id.clone())));
    assert!(a.wants().is_empty(), "asked once, not every frame");
    a.fetch_failed(&id, true);
    assert!(a.wants().is_empty(), "not again on the next frame");
    let prs = a.section().prs.clone();
    a.synced(6, prs, 23);
    assert_eq!(a.wants(), vec![Effect::FetchDiff(id)], "but again after the next sync");
}

#[test]
fn n_and_b_step_through_files_and_stop_at_the_ends() {
    let mut a = opened();
    keys(&mut a, "b");
    assert_eq!(d(&a).file, 0, "nothing before the first");
    keys(&mut a, "nnn");
    assert_eq!((d(&a).file, d(&a).focus), (3, Pane::Diff));
    for _ in 0..20 {
        keys(&mut a, "n");
    }
    assert_eq!(d(&a).file, 13, "the last file is a floor");
    keys(&mut a, "b");
    assert_eq!(d(&a).file, 12);
}

#[test]
fn the_file_list_filters_by_typing_and_opens_with_enter() {
    let mut a = opened();
    keys(&mut a, "F");
    assert!(d(&a).picker.is_some());
    let text: String = a.lines(120, 40).iter().flatten().map(|s| s.text.clone()).collect();
    assert!(text.contains("14 of 14 files") && text.contains("src/ledger/migrate.ts"));
    keys(&mut a, "part1");
    a.key(Key::Down, Instant::now());
    a.key(Key::Enter, Instant::now());
    // "part1" matches part10 to part14; the second of those is part11.
    assert!(d(&a).picker.is_none());
    assert_eq!(a.diffs.values().next().unwrap()[d(&a).file].path, "src/ledger/part11.ts");
    keys(&mut a, "F");
    a.key(Key::Esc, Instant::now());
    assert!(d(&a).picker.is_none() && a.detail.is_some(), "Esc closes the list, not the view");
}

#[test]
fn a_check_that_finishes_without_the_pr_updating_refetches_the_detail() {
    let mut a = opened();
    let mut prs = a.section().prs.clone();
    prs[0].checks.running += 1;
    let total = prs.len();
    a.synced(6, prs.clone(), total);
    let fx = a.wants();
    assert!(fx.iter().any(|e| matches!(e, Effect::FetchDetail(id) if *id == prs[0].id)), "{fx:?}");
}
