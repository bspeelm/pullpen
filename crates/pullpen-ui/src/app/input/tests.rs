use std::time::{Duration, Instant};

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
    a
}

fn press(a: &mut App, keys: &[Key]) -> Vec<Effect> {
    keys.iter().flat_map(|k| a.key(*k, Instant::now())).collect()
}

fn text(a: &mut App, t: &str) {
    for c in t.chars() {
        a.key(Key::Char(c), Instant::now());
    }
}

#[test]
fn gg_goes_to_the_top_and_a_lone_g_expires() {
    let mut a = app();
    press(&mut a, &[Key::Char('G')]);
    assert_eq!(a.section().sel, 22);
    press(&mut a, &[Key::Char('g'), Key::Char('g')]);
    assert_eq!(a.section().sel, 0);

    let t0 = Instant::now();
    a.key(Key::Char('j'), t0);
    a.key(Key::Char('g'), t0);
    a.key(Key::Char('g'), t0 + PENDING + Duration::from_millis(1));
    assert_eq!(a.section().sel, 1, "the second g came too late to make gg");
}

#[test]
fn any_other_key_at_a_confirm_cancels_and_is_discarded() {
    let mut a = app();
    press(&mut a, &[Key::Char('x')]);
    assert!(matches!(a.input, Input::Confirm(_)));
    let fx = press(&mut a, &[Key::Char('j')]);
    assert!(fx.is_empty());
    assert!(matches!(a.input, Input::Normal));
    assert_eq!(a.section().sel, 0, "the j was discarded, not acted on");
}

#[test]
fn filter_fades_then_enter_hides_then_esc_clears() {
    let mut a = app();
    press(&mut a, &[Key::Char('/')]);
    text(&mut a, "ledger");
    assert_eq!(a.section().visible().len(), 23, "typing only fades non-matches");
    assert_eq!(a.section().selected().unwrap().id.number, 4821);
    press(&mut a, &[Key::Enter]);
    assert_eq!(a.section().visible().len(), 3);
    press(&mut a, &[Key::Char('n')]);
    assert_eq!(a.section().selected().unwrap().id.number, 1199);
    press(&mut a, &[Key::Esc]);
    assert_eq!(a.section().visible().len(), 23);
}

#[test]
fn space_marks_and_moves_on_and_v_marks_the_range() {
    let mut a = app();
    press(&mut a, &[Key::Char(' ')]);
    assert_eq!((a.section().marks.len(), a.section().sel), (1, 1));
    press(&mut a, &[Key::Char('j'), Key::Char('j'), Key::Char('V')]);
    assert_eq!(a.section().marks.len(), 4, "rows 0 to 3");
    press(&mut a, &[Key::Esc]);
    assert!(a.section().marks.is_empty());
}

#[test]
fn tab_fixes_a_typo() {
    let mut a = app();
    press(&mut a, &[Key::Char(':')]);
    text(&mut a, "mrege #812");
    press(&mut a, &[Key::Enter, Key::Tab]);
    match &a.input {
        Input::Command(e) => assert_eq!(e.text, "merge #812"),
        other => panic!("expected the command line back, got {other:?}"),
    }
}

#[test]
fn section_command_takes_short_names() {
    let mut a = app();
    press(&mut a, &[Key::Char(':')]);
    text(&mut a, "section armed");
    press(&mut a, &[Key::Enter]);
    assert_eq!(a.active, 3);
}

#[test]
fn a_batch_skips_the_ineligible_by_name() {
    let mut a = app();
    for i in [9, 11] {
        a.section_mut().sel = i;
        press(&mut a, &[Key::Char(' ')]);
    }
    press(&mut a, &[Key::Char('M'), Key::Char('y')]);
    let line: String = a.message.as_ref().unwrap().line.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(line, "✓ Merged 0 PRs   skipped #1203: draft · skipped #143: test failing");
}

#[test]
fn repeated_commands_enter_history_once() {
    let mut a = app();
    for _ in 0..2 {
        press(&mut a, &[Key::Char(':')]);
        text(&mut a, "refresh");
        press(&mut a, &[Key::Enter]);
    }
    assert_eq!(a.history, vec!["refresh".to_string()]);
}

#[test]
fn ctrl_r_hash_inserts_the_selected_ref() {
    let mut a = app();
    press(&mut a, &[Key::Char('j'), Key::Char(':')]);
    text(&mut a, "approve ");
    press(&mut a, &[Key::Ctrl('r'), Key::Char('#')]);
    match &a.input {
        Input::Command(e) => assert_eq!(e.text, "approve #812"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_prompt_about_a_pr_that_merged_meanwhile_goes_away() {
    let mut a = app();
    a.section_mut().sel = 1;
    let fx = press(&mut a, &[Key::Char('M'), Key::Char('y'), Key::Char('x')]);
    assert!(matches!(a.input, Input::Confirm(_)), "close #812 asked while its merge runs");
    for e in fx {
        if let Effect::Run(job) = e {
            a.job_done(pullpen_core::JobResult { job, outcome: pullpen_core::Outcome::Done, note: None });
        }
    }
    assert!(matches!(a.input, Input::Normal));
}
