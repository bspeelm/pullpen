use std::time::Instant;

use pullpen_core::time::Timestamp;
use pullpen_core::{Pipeline, PipelineState, Pr, PrState};
use pullpen_fixtures as fx;

use super::*;
use crate::app::SectionState;
use crate::glyphs::Glyphs;
use crate::keymap::Key;
use crate::paint::Theme;
use crate::palette::{Palette, Tier};

const MERGED: usize = 5;

fn app() -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    let mut a = App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW);
    a.active = MERGED;
    a
}

fn landed(n: u64, hours_ago: i64, state: PipelineState) -> Pr {
    let mut p = fx::by_number(n).unwrap();
    p.state =
        PrState::Merged { at: Timestamp(fx::NOW.0 - hours_ago * 3600), method: pullpen_core::MergeMethod::Squash };
    let job = if state == PipelineState::Passed { "" } else { "deploy" };
    p.post_merge = Some(Pipeline {
        state,
        passed: 1,
        total: 2,
        job: job.into(),
        url: format!("https://ci.example/{n}"),
        branch: None,
    });
    p
}

/// The base branch has passed since, so the failure is history.
fn fixed(mut p: Pr) -> Pr {
    if let Some(pl) = p.post_merge.as_mut() {
        pl.branch = Some(PipelineState::Passed);
    }
    p
}

fn numbers(a: &App) -> Vec<u64> {
    a.sections[MERGED].prs.iter().map(|p| p.id.number).collect()
}

/// Types `:cmd` and Enter, as a person would.
fn command(a: &mut App, cmd: &str) -> Vec<Effect> {
    let keys = std::iter::once(':').chain(cmd.chars()).map(Key::Char).chain([Key::Enter]);
    keys.flat_map(|k| a.key(k, Instant::now())).collect()
}

fn notes(a: &mut App) -> Vec<String> {
    a.wants().into_iter().filter_map(|e| if let Effect::Notify(t) = e { Some(t) } else { None }).collect()
}

#[test]
fn a_day_old_pass_leaves_but_a_failure_stays_until_cleared() {
    let mut a = app();
    let rows = vec![landed(4815, 30, PipelineState::Failed), landed(1199, 30, PipelineState::Passed)];
    a.synced(MERGED, rows, 2);
    assert_eq!(numbers(&a), [4815], "the old pass left; the old failure did not");
    assert_eq!(a.sections[MERGED].total, 1, "the tab counts what is shown");

    let fx = command(&mut a, "clear");
    assert!(fx.iter().any(|e| matches!(e, Effect::SaveCleared)));
    assert!(numbers(&a).is_empty());
    a.synced(MERGED, vec![landed(4815, 31, PipelineState::Failed)], 1);
    assert!(numbers(&a).is_empty(), "a cleared failure does not come back with the next sync");
}

#[test]
fn clear_keeps_what_is_still_running() {
    let mut a = app();
    a.synced(MERGED, vec![landed(812, 1, PipelineState::Running), landed(1199, 1, PipelineState::Passed)], 2);
    command(&mut a, "clear");
    assert_eq!(numbers(&a), [812]);
}

#[test]
fn a_pipeline_seen_running_rings_when_it_breaks_the_branch_and_again_when_fixed() {
    let mut a = app();
    a.synced(MERGED, vec![landed(812, 1, PipelineState::Running)], 1);
    assert!(notes(&mut a).is_empty());
    a.synced(MERGED, vec![landed(812, 1, PipelineState::Failed)], 1);
    assert_eq!(notes(&mut a), ["main is failing after #812: deploy"]);
    assert_eq!(a.sections[MERGED].total, 1, "the tab counts what needs attention");
    a.synced(MERGED, vec![fixed(landed(812, 1, PipelineState::Failed))], 1);
    assert_eq!(notes(&mut a), ["main passes again after #812"]);
    assert_eq!(a.sections[MERGED].total, 0);
}

#[test]
fn a_failure_fixed_since_is_done_and_leaves_with_the_rest() {
    let mut a = app();
    a.synced(MERGED, vec![fixed(landed(4815, 30, PipelineState::Failed)), landed(4802, 30, PipelineState::Failed)], 2);
    assert_eq!(numbers(&a), [4802], "the fixed one left after a day; the one still failing did not");
    assert_eq!(a.broken(), 1);
}

#[test]
fn d_dismisses_the_selected_failure_for_good() {
    let mut a = app();
    a.synced(MERGED, vec![landed(4802, 30, PipelineState::Failed), landed(812, 1, PipelineState::Running)], 2);
    let fx = a.key(Key::Char('D'), Instant::now());
    assert!(fx.iter().any(|e| matches!(e, Effect::SaveCleared)));
    assert_eq!(numbers(&a), [812]);
    assert_eq!(a.broken(), 0);
    a.active = 2;
    a.key(Key::Char('D'), Instant::now());
    assert!(a.message.is_some(), "D elsewhere says where it works instead of doing nothing silently");
}

#[test]
fn o_opens_the_pipeline_rather_than_the_pr() {
    let mut a = app();
    a.synced(MERGED, vec![landed(812, 1, PipelineState::Running)], 1);
    let fx = a.key(Key::Char('o'), Instant::now());
    assert!(matches!(fx.as_slice(), [Effect::Open(u)] if u == "https://ci.example/812"), "{fx:?}");
}

#[test]
fn the_summary_leads_with_what_needs_attention() {
    let a = app();
    let line: String = a.lines(120, 40)[39].iter().map(|s| s.text.as_str()).collect();
    assert_eq!(line.trim_end(), "✗ 1 needs attention   ·   2 running   ·   4 done");
}

#[test]
fn the_tab_title_carries_what_needs_attention() {
    let mut a = app();
    assert_eq!(a.title(), "pullpen ✗1", "the fixtures hold one broken branch");
    a.synced(MERGED, vec![fixed(landed(4815, 1, PipelineState::Failed))], 1);
    assert_eq!(a.title(), "pullpen");
}
