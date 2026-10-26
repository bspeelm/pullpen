//! The `Forge` trait and its GitHub and GitLab clients (PLAN.md §4).

pub mod auth;
pub mod github;
pub mod gitlab;
mod http;

use std::collections::HashMap;

use async_trait::async_trait;
use pullpen_core::diff::FileDiff;
use pullpen_core::query::Query;
use pullpen_core::time::Timestamp;
use pullpen_core::{Action, Capabilities, Forge as Kind, Job, MergeMethod, Pr, PrDetail, PrId};

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
    /// What the forge says matched and pullpen may see, which can exceed `prs`.
    pub total: usize,
    /// Rows the forge counted but would not show, and why: the user should
    /// know their list is incomplete, not find out by noticing.
    pub hidden: Option<String>,
}

/// Somewhere PRs can be watched. The path is what searches take; the name
/// is what the forge's own pages show, which on GitLab can differ (a group
/// at `a5434` named "AIP").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub path: String,
    pub name: String,
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
    /// Where the signed-in user can watch PRs: on GitHub their own account
    /// and their orgs, on GitLab the groups they belong to.
    async fn places(&self) -> Result<Vec<Place>>;
    /// PRs matching the server-side part of `q`; the caller applies the
    /// client-side terms.
    async fn search(&self, q: &Query, me: &str, limit: usize) -> Result<Page>;
    /// What each repo seen so far allows, keyed `host/repo`.
    fn capabilities(&self) -> HashMap<String, Capabilities>;
    /// Whether the source branch was deleted with it.
    async fn merge(&self, id: &PrId, m: MergeMethod) -> Result<bool>;
    async fn arm(&self, id: &PrId, m: MergeMethod) -> Result<()>;
    async fn disarm(&self, id: &PrId) -> Result<()>;
    async fn approve(&self, id: &PrId) -> Result<()>;
    async fn comment(&self, id: &PrId, body: &str) -> Result<()>;
    async fn close(&self, id: &PrId) -> Result<()>;
    /// Brings the branch up to date with its base.
    async fn update(&self, id: &PrId) -> Result<()>;
    /// Description, checks and timeline, for the preview and detail view.
    async fn detail(&self, id: &PrId) -> Result<PrDetail>;
    async fn diff(&self, id: &PrId) -> Result<Vec<FileDiff>>;
}

/// `1m 52s`, `38s`, `1h 4m`: how long a check took.
pub fn duration(from: Timestamp, to: Timestamp) -> String {
    let s = (to.0 - from.0).max(0);
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m {:02}s", s / 60, s % 60),
        _ => format!("{}h {}m", s / 3600, s % 3600 / 60),
    }
}

/// Runs a job's action, and says what else happened. Opening and checking
/// out are local, not forge calls.
pub async fn run(f: &dyn Forge, job: &Job) -> Result<Option<String>> {
    let id = &job.target;
    let done = match &job.action {
        Action::Merge { method } => {
            return Ok(f.merge(id, *method).await?.then(|| "source branch deleted".to_string()));
        }
        Action::Arm { method } => f.arm(id, *method).await,
        Action::Disarm => f.disarm(id).await,
        Action::Approve => f.approve(id).await,
        Action::Comment(body) => f.comment(id, body).await,
        Action::Close => f.close(id).await,
        Action::Update => f.update(id).await,
        Action::Checkout { .. } | Action::Open | Action::Refresh { .. } => Ok(()),
    };
    done.map(|()| None)
}

#[cfg(test)]
mod tests;
