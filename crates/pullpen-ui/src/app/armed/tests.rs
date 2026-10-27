use std::time::Instant;

use pullpen_core::time::Timestamp;
use pullpen_core::{AutoMerge, CheckRun, JobResult, MergeMethod, Outcome, Pr, PrState};
use pullpen_fixtures as fx;

use super::*;
use crate::app::SectionState;
use crate::glyphs::Glyphs;
use crate::keymap::Key;
use crate::paint::Theme;
use crate::palette::{Palette, Tier};

const ARMED: usize = 2;

fn app() -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    let mut a = App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW);
    a.active = ARMED;
    a
}

fn armed(n: u64) -> Pr {
    let mut p = fx::by_number(n).unwrap();
    p.auto_merge = Some(AutoMerge { method: MergeMethod::Squash, armed_by: "you".into(), armed_at: fx::NOW });
    p
}

fn notes(a: &mut App) -> Vec<String> {
    a.wants().into_iter().filter_map(|e| if let Effect::Notify(t) = e { Some(t) } else { None }).collect()
}

#[test]
fn armed_then_running_then_merged_rings_once() {
    let mut a = app();
    let running = armed(377);
    a.synced(ARMED, vec![running.clone()], 1);
    assert_eq!(a.group_of(&a.sections[ARMED].prs[0]), Some(Group::Waiting));
    a.synced(ARMED, vec![running.clone()], 1);
    assert!(notes(&mut a).is_empty(), "arming and staying armed ring nothing");
    let mut merged = running;
    merged.auto_merge = None;
    merged.state = PrState::Merged { at: Timestamp(fx::NOW.0 + 60), method: MergeMethod::Squash };
    a.synced(ARMED, vec![merged.clone()], 1);
    assert_eq!(notes(&mut a), vec!["#377 merged".to_string()]);
    a.synced(ARMED, vec![merged], 1);
    assert!(notes(&mut a).is_empty(), "exactly one notification");
    assert_eq!(a.sections[ARMED].total, 0, "merged rows are not counted as armed");
}

#[test]
fn a_waiting_pr_that_fails_needs_attention_and_says_so() {
    let mut a = app();
    let p = armed(377);
    a.synced(ARMED, vec![p.clone()], 1);
    let mut failing = p;
    failing.checks.failed.push(CheckRun { name: "e2e".into(), detail: String::new() });
    a.synced(ARMED, vec![failing.clone()], 1);
    assert_eq!(notes(&mut a), vec!["#377 needs attention".to_string()]);
    assert_eq!(a.group_of(&failing), Some(Group::Attention));
}

#[test]
fn the_forge_turning_auto_merge_off_is_attention() {
    let mut a = app();
    let p = armed(377);
    a.synced(ARMED, vec![p.clone()], 1);
    let mut off = p;
    off.auto_merge = None;
    a.synced(ARMED, vec![off.clone()], 1);
    assert_eq!(notes(&mut a), vec!["#377 auto-merge was turned off".to_string()]);
    assert_eq!(a.group_of(&off), Some(Group::Attention));
}

#[test]
fn a_merge_pullpen_made_rings_nothing() {
    let mut a = app();
    let p = armed(812);
    a.synced(ARMED, vec![p.clone()], 1);
    let fx = [Key::Char('M'), Key::Char('y')].iter().flat_map(|k| a.key(*k, Instant::now())).collect::<Vec<_>>();
    for e in fx {
        if let Effect::Run(job) = e {
            a.job_done(JobResult { job, outcome: Outcome::Done, note: None });
        }
    }
    let mut merged = p;
    merged.state = PrState::Merged { at: fx::NOW, method: MergeMethod::Squash };
    a.synced(ARMED, vec![merged], 1);
    assert!(notes(&mut a).is_empty());
}

#[test]
fn rows_are_grouped_attention_first_and_merges_are_counted_from_merged() {
    let mut a = app();
    let mut failing = armed(2290);
    failing.checks.failed = vec![CheckRun { name: "e2e".into(), detail: String::new() }];
    a.synced(ARMED, vec![armed(377), failing], 2);
    let order: Vec<u64> = a.sections[ARMED].prs.iter().map(|p| p.id.number).collect();
    assert_eq!(order, [2290, 377]);
    let summary: String = a.armed_summary(120).iter().map(|s| s.text.as_str()).collect();
    assert!(summary.starts_with("◆ 7 merged in the last day → Merged"), "{summary}");
    let screen: Vec<String> = a.lines(120, 40).iter().map(|l| l.iter().map(|s| s.text.as_str()).collect()).collect();
    assert!(!screen.iter().any(|l| l.trim_start().starts_with("merged")), "no merged group: Merged lists them");
}
