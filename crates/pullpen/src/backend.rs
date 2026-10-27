//! What the event loop talks to for anything off the UI thread: running
//! jobs, syncing sections, and the answers to both.

use std::collections::HashMap;

use pullpen_core::diff::FileDiff;
use pullpen_core::time::Timestamp;
use pullpen_core::{Capabilities, Forge, Job, JobResult, Pr, PrDetail, PrId};

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
    fn messages(&mut self) -> Vec<Msg>;
    /// `synced 40s`, for the status line.
    fn sync_label(&self) -> String;
    /// What every "12m ago" counts from.
    fn now(&self) -> Timestamp;
    fn open(&mut self, url: &str) -> Result<(), String>;
}
