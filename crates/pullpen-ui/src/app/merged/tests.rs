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

const MERGED: usize = 4;

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

fn screen(a: &App) -> Vec<String> {
    a.lines(120, 40).iter().map(|l| l.iter().map(|s| s.text.as_str()).collect()).collect()
}

#[test]
fn every_merge_in_the_window_stays_newest_first() {
    let mut a = app();
    let rows = vec![
        landed(4815, 30, PipelineState::Failed),
        landed(1199, 50, PipelineState::Passed),
        landed(812, 1, PipelineState::Running),
    ];
    a.synced(MERGED, rows, 3);
    assert_eq!(numbers(&a), [812, 4815, 1199], "a day-old pass is still history");
    assert_eq!(a.sections[MERGED].total, 3, "the tab counts what merged");
}

#[test]
fn clear_drops_finished_rows_and_keeps_what_runs_or_fails() {
    let mut a = app();
    let rows = || {
        vec![
            landed(812, 1, PipelineState::Running),
            landed(1199, 1, PipelineState::Passed),
            landed(4815, 2, PipelineState::Failed),
            fixed(landed(4802, 3, PipelineState::Failed)),
        ]
    };
    a.synced(MERGED, rows(), 4);
    let fx = command(&mut a, "clear");
    assert!(fx.iter().any(|e| matches!(e, Effect::SaveCleared)));
    assert_eq!(numbers(&a), [812, 4815]);
    a.synced(MERGED, rows(), 4);
    assert_eq!(numbers(&a), [812, 4815], "cleared rows do not come back with the next sync");
}

#[test]
fn a_failure_reads_as_failed_whatever_the_branch_does_since() {
    let mut a = app();
    let mut broke = landed(4815, 1, PipelineState::Failed);
    broke.post_merge.as_mut().unwrap().branch = Some(PipelineState::Failed);
    a.synced(MERGED, vec![broke], 1);
    assert!(screen(&a).iter().any(|l| l.contains("failed  deploy")), "Merged is a record; Deploys says the branch");
}

#[test]
fn o_opens_the_pipeline_and_d_points_to_deploys() {
    let mut a = app();
    a.synced(MERGED, vec![landed(812, 1, PipelineState::Running)], 1);
    let fx = a.key(Key::Char('o'), Instant::now());
    assert!(matches!(fx.as_slice(), [Effect::Open(u)] if u == "https://ci.example/812"), "{fx:?}");
    a.key(Key::Char('D'), Instant::now());
    assert!(a.message.as_ref().is_some_and(|m| m.line.iter().any(|s| s.text.contains("Deploys"))));
}

#[test]
fn the_summary_counts_and_names_the_keys() {
    let a = app();
    let last = screen(&a).pop().unwrap();
    assert!(last.starts_with("7 merged") && last.trim_end().ends_with("w pipeline   :clear drop finished"), "{last}");
}
