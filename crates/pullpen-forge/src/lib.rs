//! The `Forge` trait and its GitHub and GitLab clients (PLAN.md §4).

pub mod auth;
pub mod github;
pub mod gitlab;
mod http;

use std::collections::HashMap;

use async_trait::async_trait;
use pullpen_core::query::Query;
use pullpen_core::{Action, Capabilities, Forge as Kind, Job, MergeMethod, Pr, PrId};

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("not signed in to {0}; set a token or run its CLI's login")]
    Auth(String),
    #[error("rate limited; retrying in {retry_in}s")]
    RateLimited { retry_in: u64 },
    #[error("{host} {status}")]
    Server { host: String, status: u16 },
    /// The forge understood and said no. The text is the forge's own.
    #[error("{0}")]
    Refused(String),
    #[error("{0}")]
    Network(String),
    #[error("unexpected answer from the forge: {0}")]
    Decode(String),
}

impl Error {
    /// Errors that go on the status line and are retried with backoff, as
    /// opposed to ones a command reports (PLAN.md §4.2).
    pub fn transient(&self) -> bool {
        matches!(self, Error::RateLimited { .. } | Error::Server { .. } | Error::Network(_))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// One section's worth of PRs from one forge.
pub struct Page {
    pub prs: Vec<Pr>,
    /// What the forge says matched, which can exceed `prs`.
    pub total: usize,
}

/// Who counts as an agent: these logins, plus any login ending in `[bot]`
/// and any account the forge itself marks as a bot (PLAN.md §3).
#[derive(Clone, Debug, Default)]
pub struct Agents(pub Vec<String>);

impl Agents {
    pub fn default_logins() -> Vec<String> {
        ["claude", "codex", "renovate", "dependabot", "devin", "copilot", "github-actions"].map(String::from).to_vec()
    }
    pub fn is_agent(&self, login: &str, forge_says_bot: bool) -> bool {
        let bare = login.trim_end_matches("[bot]");
        forge_says_bot || login.ends_with("[bot]") || self.0.iter().any(|a| a.eq_ignore_ascii_case(bare))
    }
}

#[async_trait]
pub trait Forge: Send + Sync {
    fn kind(&self) -> Kind;
    fn host(&self) -> &str;
    /// The signed-in login, which is what `@me` means.
    async fn viewer(&self) -> Result<String>;
    /// PRs matching the server-side part of `q`; the caller applies the
    /// client-side terms.
    async fn search(&self, q: &Query, me: &str, limit: usize) -> Result<Page>;
    /// What each repo seen so far allows, keyed `host/repo`.
    fn capabilities(&self) -> HashMap<String, Capabilities>;
    async fn merge(&self, id: &PrId, m: MergeMethod) -> Result<()>;
    async fn arm(&self, id: &PrId, m: MergeMethod) -> Result<()>;
    async fn disarm(&self, id: &PrId) -> Result<()>;
    async fn approve(&self, id: &PrId) -> Result<()>;
    async fn comment(&self, id: &PrId, body: &str) -> Result<()>;
    async fn close(&self, id: &PrId) -> Result<()>;
}

/// Runs a job's action. Opening and checking out are local, not forge calls.
pub async fn run(f: &dyn Forge, job: &Job) -> Result<()> {
    let id = &job.target;
    match &job.action {
        Action::Merge { method } => f.merge(id, *method).await,
        Action::Arm { method } => f.arm(id, *method).await,
        Action::Disarm => f.disarm(id).await,
        Action::Approve => f.approve(id).await,
        Action::Comment(body) => f.comment(id, body).await,
        Action::Close => f.close(id).await,
        Action::Checkout | Action::Open | Action::Refresh { .. } => Ok(()),
    }
}

#[cfg(test)]
mod tests;
