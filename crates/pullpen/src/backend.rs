//! What the event loop talks to for anything off the UI thread: running
//! jobs, syncing sections, and the answers to both.

use std::collections::HashMap;

use pullpen_core::diff::FileDiff;
use pullpen_core::time::Timestamp;
use pullpen_core::{Capabilities, Forge, Job, JobResult, Pr, PrDetail, PrId, Run};

pub enum Msg {
    Job(JobResult),
    Synced {
        section: usize,
        prs: Vec<Pr>,
        total: usize,
    },
    Down(Forge, String),
    Up(Forge),
    Caps(HashMap<String, Capabilities>),
    Detail(PrId, PrDetail),
    Diff(PrId, Vec<FileDiff>),
    Runs(PrId, Vec<Run>),
    /// A job's log, by PR and job id, as the forge sent it.
    Log(PrId, String, String),
    /// A runs (`log: false`) or log fetch that failed, with why.
    Unwatched {
        log: bool,
        why: String,
    },
    /// A detail (`diff: false`) or diff fetch that failed, with why.
    Unfetched {
        id: PrId,
        diff: bool,
        why: String,
    },
}

pub trait Backend {
    fn run(&mut self, job: Job);
    /// Refetch one section now, or every section.
    fn refresh(&mut self, section: Option<usize>);
    /// Fetches a PR's detail, or its diff.
    fn fetch(&mut self, id: PrId, diff: bool);
    /// Fetches a PR's pipeline runs.
    fn runs(&mut self, id: PrId);
    /// Fetches a job's log.
    fn log(&mut self, id: PrId, job: String);
    fn messages(&mut self) -> Vec<Msg>;
    /// `synced 40s`, for the status line.
    fn sync_label(&self) -> String;
    /// What every "12m ago" counts from.
    fn now(&self) -> Timestamp;
    fn open(&mut self, url: &str) -> Result<(), String>;
}
