use std::time::{Duration, Instant};

use pullpen_core::{CheckState, Forge, PrId};
use pullpen_fixtures as fx;

use crate::app::{App, Effect, SectionState};
use crate::glyphs::Glyphs;
use crate::keymap::Key;
use crate::paint::Theme;
use crate::palette::{Palette, Tier};

const MERGED: usize = 5;

/// The Merged section with #812 selected.
fn app() -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    let mut a = App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW);
    a.active = MERGED;
    a.height = 40;
    a.sections[MERGED].sel = a.sections[MERGED].prs.iter().position(|p| p.id.number == 812).unwrap();
    a
}

fn id812() -> PrId {
    PrId { forge: Forge::GitLab, host: "gitlab.com".into(), repo: "platform/runner".into(), number: 812 }
}

fn press(a: &mut App, keys: &str) -> Vec<Effect> {
    keys.chars().flat_map(|c| a.key(if c == '\n' { Key::Enter } else { Key::Char(c) }, Instant::now())).collect()
}

/// `w`, the runs answered, then Enter on `build`, its log answered.
fn in_job() -> App {
    let mut a = app();
    press(&mut a, "w");
    a.runs_loaded(id812(), fx::runs(812));
    press(&mut a, "\n");
    a.log_loaded(id812(), "79".into(), fx::LOG);
    a
}

fn fetches(fx: &[Effect]) -> Vec<String> {
    fx.iter()
        .filter_map(|e| match e {
            Effect::FetchRuns(id) => Some(format!("runs #{}", id.number)),
            Effect::FetchLog(_, job) => Some(format!("log {job}")),
            _ => None,
        })
        .collect()
}

#[test]
fn w_enter_and_q_walk_down_to_a_job_and_back_to_where_w_was_pressed() {
    let mut a = app();
    press(&mut a, "\nw");
    assert!(a.detail.is_some() && a.pipe.is_some(), "w from the detail view opens over it");
    a.runs_loaded(id812(), fx::runs(812));
    assert_eq!(a.pipe.as_ref().unwrap().sel, 2, "the cursor starts on the running job");
    press(&mut a, "\n");
    assert_eq!(a.open_job().map(|(_, j, _)| j.name.as_str()), Some("build"));
    press(&mut a, "q");
    assert!(a.pipe.as_ref().is_some_and(|p| p.job.is_none()), "q leaves the job for the pipeline");
    press(&mut a, "q");
    assert!(a.pipe.is_none() && a.detail.is_some(), "and the pipeline for the detail view");

    let mut a = app();
    press(&mut a, ":watch #812\n");
    assert_eq!(a.pipe.as_ref().map(|p| p.id.number), Some(812), ":watch opens it too");
}

#[test]
fn a_running_pipeline_is_asked_for_every_five_seconds_and_a_finished_one_not_at_all() {
    let mut a = app();
    press(&mut a, "w");
    let t0 = Instant::now();
    assert_eq!(fetches(&a.watch(t0)), ["runs #812"]);
    assert!(fetches(&a.watch(t0 + Duration::from_secs(9))).is_empty(), "not while an answer is on its way");
    a.runs_loaded(id812(), fx::runs(812));
    assert!(fetches(&a.watch(t0 + Duration::from_secs(4))).is_empty());
    assert_eq!(fetches(&a.watch(t0 + Duration::from_secs(5))), ["runs #812"]);

    let mut done = fx::runs(812);
    done[0].state = CheckState::Passed;
    a.runs_loaded(id812(), done);
    assert!(fetches(&a.watch(t0 + Duration::from_secs(60))).is_empty(), "everything finished: nothing to watch");
    press(&mut a, "r");
    assert_eq!(fetches(&a.watch(t0 + Duration::from_secs(61))), ["runs #812"], "r asks anyway");
    press(&mut a, "q");
    assert!(a.watch(t0 + Duration::from_secs(120)).is_empty(), "closed: nothing is polled");
}

#[test]
fn a_running_gitlab_jobs_log_is_asked_for_again_and_a_finished_one_once() {
    let mut a = in_job();
    let t0 = Instant::now();
    a.poll.runs_waiting = true;
    a.poll.log_asked = Some(t0);
    assert!(fetches(&a.watch(t0 + Duration::from_secs(2))).is_empty());
    assert_eq!(fetches(&a.watch(t0 + Duration::from_secs(5))), ["log 79"]);
    a.log_loaded(id812(), "79".into(), fx::LOG);

    let mut done = fx::runs(812);
    done[0].jobs[2].state = CheckState::Passed;
    a.runs_loaded(id812(), done);
    a.poll.runs_waiting = true;
    assert_eq!(fetches(&a.watch(t0 + Duration::from_secs(6))), ["log 79"], "the whole log, once, when it finishes");
    a.log_loaded(id812(), "79".into(), fx::LOG);
    assert!(fetches(&a.watch(t0 + Duration::from_secs(60))).is_empty());
}

#[test]
fn github_says_why_a_running_job_has_no_log_yet() {
    let a = app();
    let job = fx::runs(812)[0].jobs[2].clone();
    let github = PrId { forge: Forge::GitHub, ..id812() };
    assert!(a.no_log(&github, &job).is_some_and(|n| n.contains("when the job finishes")));
    assert_eq!(a.no_log(&id812(), &job), None, "GitLab serves a running job's log");
}

#[test]
fn scrolling_up_stops_following_and_g_resumes_it() {
    let mut a = in_job();
    let follow = |a: &App| a.open_job().unwrap().2.follow;
    assert!(follow(&a));
    press(&mut a, "gg");
    assert!(!follow(&a));
    press(&mut a, "G");
    assert!(follow(&a));
}

#[test]
fn e_finds_the_warning_free_error_and_search_jumps_to_a_match() {
    let mut a = in_job();
    press(&mut a, "e");
    assert!(a.message.is_some(), "the fixture log has no error line, and e says so");
    press(&mut a, "gg/golang.org\n");
    let v = &a.open_job().unwrap().2;
    assert_eq!(v.search.as_deref(), Some("golang.org"));
    assert!(!v.follow);
    let top = v.scroll;
    assert!(a.current_log().unwrap().lines[top..].iter().any(|l| l.text.contains("golang.org")));
}

#[test]
fn h_and_enter_jump_the_log_to_a_step() {
    let mut a = in_job();
    // Short enough that the 14-line log scrolls.
    a.height = 14;
    press(&mut a, "hj\n");
    let v = &a.open_job().unwrap().2;
    assert!(!v.steps, "Enter hands focus back to the log");
    assert_eq!(a.current_log().unwrap().lines[v.scroll].text, "Preparing environment");
}

#[test]
fn a_run_finishing_refetches_the_prs_detail() {
    let mut a = app();
    press(&mut a, "w");
    a.details.insert(id812(), fx::detail(812));
    a.runs_loaded(id812(), fx::runs(812));
    assert!(a.details.contains_key(&id812()));
    let mut done = fx::runs(812);
    done[0].state = CheckState::Passed;
    a.runs_loaded(id812(), done);
    assert!(!a.details.contains_key(&id812()), "the overview's checks would disagree with the pipeline");
}
