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

const DEPLOYS: usize = 3;

fn app() -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    let mut a = App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW);
    a.active = DEPLOYS;
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
    a.sections[DEPLOYS].prs.iter().map(|p| p.id.number).collect()
}

fn notes(a: &mut App) -> Vec<String> {
    a.wants().into_iter().filter_map(|e| if let Effect::Notify(t) = e { Some(t) } else { None }).collect()
}

fn screen(a: &App) -> Vec<String> {
    a.lines(120, 40).iter().map(|l| l.iter().map(|s| s.text.as_str()).collect()).collect()
}

/// #4815, #4802 and #4818 are all acme/api into main.
#[test]
fn a_branch_is_one_lane_headed_by_its_newest_merge_with_a_pipeline() {
    let mut a = app();
    let mut none = landed(4818, 0, PipelineState::Passed);
    none.post_merge = None;
    let rows = vec![
        landed(4802, 5, PipelineState::Passed),
        landed(4815, 2, PipelineState::Failed),
        none,
        landed(812, 1, PipelineState::Running),
    ];
    a.synced(DEPLOYS, rows, 4);
    assert_eq!(numbers(&a), [812, 4815], "deploying, then failing; #4818 ran no pipeline");
}

#[test]
fn lanes_are_deploying_failing_then_deployed_and_a_fixed_branch_is_deployed() {
    let mut a = app();
    let rows = vec![
        fixed(landed(4815, 2, PipelineState::Failed)),
        landed(812, 1, PipelineState::Running),
        landed(1199, 3, PipelineState::Passed),
        landed(2290, 4, PipelineState::Failed),
    ];
    a.synced(DEPLOYS, rows, 4);
    assert_eq!(numbers(&a), [812, 2290, 4815, 1199]);
    assert_eq!(a.sections[DEPLOYS].total, 1, "the tab counts failing lanes");
    assert_eq!((a.broken(), a.title().as_str()), (1, "pullpen ✗1"));
    let s = screen(&a);
    assert!(
        s.iter().any(|l| l.trim_start().starts_with("deploying"))
            && s.iter().any(|l| l.contains("fixed  main passes now"))
    );
}

#[test]
fn a_lane_rings_when_it_breaks_its_branch_and_again_when_fixed() {
    let mut a = app();
    a.synced(DEPLOYS, vec![landed(812, 1, PipelineState::Running)], 1);
    assert!(notes(&mut a).is_empty());
    a.synced(DEPLOYS, vec![landed(812, 1, PipelineState::Failed)], 1);
    assert_eq!(notes(&mut a), ["main is failing after #812: deploy"]);
    a.synced(DEPLOYS, vec![fixed(landed(812, 1, PipelineState::Failed))], 1);
    assert_eq!(notes(&mut a), ["main passes again after #812"]);
    assert_eq!(a.title(), "pullpen");
}

#[test]
fn d_dismisses_a_failing_lane_quietly_until_a_newer_merge_lands() {
    let mut a = app();
    a.synced(DEPLOYS, vec![landed(4815, 2, PipelineState::Failed), landed(812, 1, PipelineState::Running)], 2);
    notes(&mut a);
    a.sections[DEPLOYS].sel = 1;
    let fx = a.key(Key::Char('D'), Instant::now());
    assert!(fx.iter().any(|e| matches!(e, Effect::SaveCleared)));
    assert_eq!((a.broken(), a.sections[DEPLOYS].total), (0, 0));
    assert!(screen(&a).iter().any(|l| l.contains("dismissed  deploy")));
    a.synced(DEPLOYS, vec![landed(4815, 2, PipelineState::Failed), landed(812, 1, PipelineState::Running)], 2);
    assert!(notes(&mut a).is_empty(), "dismissing is not the branch passing");
    a.active = 0;
    a.key(Key::Char('D'), Instant::now());
    assert!(a.message.is_some(), "D elsewhere says where it works");
}

#[test]
fn enter_opens_the_lanes_pipeline_and_o_the_forges_page() {
    let mut a = app();
    a.synced(DEPLOYS, vec![landed(812, 1, PipelineState::Running)], 1);
    let fx = a.key(Key::Char('o'), Instant::now());
    assert!(matches!(fx.as_slice(), [Effect::Open(u)] if u == "https://ci.example/812"), "{fx:?}");
    a.key(Key::Enter, Instant::now());
    assert_eq!(a.pipe.as_ref().map(|p| p.id.number), Some(812));
    assert!(a.detail.is_none(), "a lane is a pipeline, not a PR to read");
}

#[test]
fn the_summary_leads_with_what_needs_you_and_names_the_keys() {
    let a = app();
    let last = screen(&a).pop().unwrap();
    assert!(last.starts_with("✗ 1 failing   ·   2 deploying   ·   2 deployed"), "{last}");
    assert!(last.trim_end().ends_with("Enter pipeline   D dismiss   o open"), "{last}");
}
