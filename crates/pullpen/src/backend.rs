//! What the event loop talks to for anything off the UI thread: running
//! jobs, syncing sections, and the answers to both.

use std::collections::HashMap;

use pullpen_core::{Capabilities, Forge, Job, JobResult, Pr};

pub enum Msg {
    Job(JobResult),
    Synced { section: usize, prs: Vec<Pr>, total: usize },
    Down(Forge, String),
    Up(Forge),
    Caps(HashMap<String, Capabilities>),
}

pub trait Backend {
    fn run(&mut self, job: Job);
    /// Refetch one section now, or every section.
    fn refresh(&mut self, section: Option<usize>);
    fn messages(&mut self) -> Vec<Msg>;
    /// `synced 40s`, for the status line.
    fn sync_label(&self) -> String;
    fn open(&mut self, url: &str) -> Result<(), String>;
}
