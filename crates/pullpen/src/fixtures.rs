//! `--fixtures`: the design's data and a fake forge, offline (PLAN.md §8).

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

use pullpen_core::{Action, Job, JobResult, Outcome};
use pullpen_fixtures as fx;
use pullpen_ui::app::{App, SectionState};
use pullpen_ui::paint::Theme;

use crate::backend::{Backend, Msg};

pub fn app(theme: Theme) -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    let mut a = App::new(sections, theme, fx::NOW);
    a.active = 2;
    a.me = "you".into();
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

pub struct Fake {
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl Fake {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Fake { tx, rx }
    }
}

impl Backend for Fake {
    /// Answers after a pause long enough to see the spinner.
    fn run(&mut self, job: Job) {
        let tx = self.tx.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(700));
            let outcome = match refusal(&job) {
                Some(r) => Outcome::Failed { reason: r.into() },
                None => Outcome::Done,
            };
            // The receiver is gone only once the app has quit.
            let _ = tx.send(Msg::Job(JobResult { job, outcome }));
        });
    }

    fn refresh(&mut self, _: Option<usize>) {}

    fn messages(&mut self) -> Vec<Msg> {
        self.rx.try_iter().collect()
    }

    fn sync_label(&self) -> String {
        "fixtures".into()
    }

    fn open(&mut self, url: &str) -> Result<(), String> {
        Err(format!("fixtures: would open {url}"))
    }
}
