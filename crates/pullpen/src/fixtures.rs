//! `--fixtures`: the design's data and a fake forge, offline (PLAN.md §8).

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use pullpen_core::{Action, Job, JobResult, Outcome};
use pullpen_fixtures as fx;
use pullpen_ui::app::{App, SectionState};
use pullpen_ui::paint::Theme;

pub fn app(theme: Theme) -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    let mut a = App::new(sections, theme, fx::NOW);
    a.active = 2;
    a.me = "you".into();
    a.sync = "fixtures".into();
    for s in &a.sections {
        for p in &s.prs {
            a.details.entry(p.id.clone()).or_insert_with(|| fx::detail(p.id.number));
        }
    }
    a
}

/// What the fake forge refuses, so failures can be seen without a forge.
fn refusal(job: &Job) -> Option<&'static str> {
    match (job.target.number, &job.action) {
        (2290, Action::Approve) => {
            Some("you are the author of this PR's last commit (repo requires independent review)")
        }
        (143, Action::Merge { .. }) => Some("required status check \"test\" is failing"),
        _ => None,
    }
}

/// Answers a job after a pause long enough to see the spinner.
pub fn run(job: Job, tx: Sender<JobResult>) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(700));
        let outcome = match refusal(&job) {
            Some(r) => Outcome::Failed { reason: r.into() },
            None => Outcome::Done,
        };
        // The receiver is gone only when the app has quit; the answer has
        // nowhere to go and nothing to do.
        let _ = tx.send(JobResult { job, outcome });
    });
}
