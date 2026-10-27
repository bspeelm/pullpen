use pullpen_core::Step;
use pullpen_core::time::Timestamp;

use super::*;
use crate::views::log::parse;

fn text(l: &Line) -> String {
    l.iter().map(|x| x.text.as_str()).collect()
}

fn job(state: CheckState, steps: &[(&str, CheckState, i64)]) -> JobRun {
    JobRun {
        id: "121".into(),
        name: "test".into(),
        group: "ci".into(),
        state,
        started: Some(Timestamp(100)),
        finished: None,
        steps: steps
            .iter()
            .map(|(n, st, at)| Step { name: (*n).into(), state: *st, started: Some(Timestamp(*at)), finished: None })
            .collect(),
        url: String::new(),
        log: true,
    }
}

#[test]
fn a_github_job_says_which_step_it_is_on_or_failed_at() {
    use CheckState::*;
    let running = job(Running, &[("checkout", Passed, 100), ("cargo test", Running, 110), ("upload", Queued, 0)]);
    assert_eq!(text(&markup(&progress(&running))), "running  step 2 of 3 · cargo test");
    let failed = job(Failed, &[("checkout", Passed, 100), ("cargo test", Failed, 110)]);
    assert_eq!(text(&markup(&progress(&failed))), "failed  at cargo test");
    assert_eq!(text(&markup(&progress(&job(Running, &[])))), "running", "GitLab has no steps to name");
}

#[test]
fn github_steps_find_their_place_in_the_log_by_time() {
    use CheckState::*;
    let j = job(Passed, &[("checkout", Passed, 1_790_000_000), ("cargo test", Passed, 1_790_000_002)]);
    let log = parse(
        "2026-09-21T14:13:20Z ##[group]Run actions/checkout\n2026-09-21T14:13:21Z cloning\n2026-09-21T14:13:22Z ##[group]Run cargo test\n",
        false,
    );
    let rows: Vec<(String, Option<usize>)> =
        steps(&j, Some(&log), Timestamp(0)).into_iter().map(|x| (x.name, x.line)).collect();
    assert_eq!(rows, [("checkout".to_string(), Some(0)), ("cargo test".to_string(), Some(2))]);
    assert!(steps(&j, None, Timestamp(0)).iter().all(|x| x.line.is_none()), "no log yet: nowhere to jump");
}

#[test]
fn a_job_from_outside_the_forges_ci_has_no_steps_and_a_queued_one_no_time() {
    let mut j = job(CheckState::Queued, &[]);
    j.log = false;
    assert!(steps(&j, None, Timestamp(0)).is_empty());
    let ctx_caps = std::collections::HashMap::new();
    let ctx = Ctx { now: Timestamp(500), glyphs: crate::glyphs::Glyphs::Unicode, caps: &ctx_caps };
    let row = text(&job_row(&j, false, 120, &ctx));
    assert!(row.trim_start().starts_with("·  test") && row.contains("queued"), "{row}");
    assert!(row.trim_end().ends_with("queued"), "nothing in the time column: {row}");
}
