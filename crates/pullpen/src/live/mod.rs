//! The real forges: one sync loop per section, jobs as tasks, answers back
//! over a channel (PLAN.md §4.3).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pullpen_core::query::Query;
use pullpen_core::time::Timestamp;
use pullpen_core::{Action, Forge as Kind, Job, JobResult, Outcome, Pr, PrId, PrState, Section, Sort, View, readiness};
use pullpen_forge::{Agents, Error, Forge, auth};
use tokio::runtime::Runtime;
use tokio::sync::{Notify, OnceCell};

use crate::backend::{Backend, Msg};
use crate::config::{Config, Kind as CfgKind};
use crate::credentials::Credentials;
use crate::{cache, checkout};

/// Per section, per forge: two screens of rows. Each 10 rows cost GitHub a
/// details query, so more is slower for rows nobody scrolls to; a section
/// that matches more says so in its count.
const LIMIT: usize = 60;
const BACKOFF_MAX: u64 = 60;

struct Entry {
    forge: Arc<dyn Forge>,
    me: OnceCell<String>,
}

pub struct Live {
    rt: Runtime,
    forges: Arc<Vec<Entry>>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    wake: Vec<Arc<Notify>>,
    last: Arc<Mutex<Option<Instant>>>,
    repos: Arc<HashMap<String, String>>,
    worktree: bool,
}

impl Live {
    /// The backend, the forges it reaches, and why any configured forge
    /// could not be reached.
    /// `since` is the last visit: an armed section also keeps what merged
    /// after it.
    pub fn start(
        cfg: &Config,
        sections: &[Section],
        cache_dir: PathBuf,
        since: Timestamp,
        creds: &Credentials,
    ) -> std::io::Result<(Live, Vec<Kind>, Vec<String>)> {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?;
        let agents = Agents(cfg.agents.logins.clone());
        let explicit = !cfg.forge.is_empty();
        let (mut forges, mut kinds, mut problems) = (Vec::new(), Vec::new(), Vec::new());
        for f in cfg.forges() {
            let kind = match f.kind {
                CfgKind::Github => Kind::GitHub,
                CfgKind::Gitlab => Kind::GitLab,
            };
            let token = match auth::token(kind, f.host(), creds.get(f.host()), f.token_cmd.as_deref(), |k| {
                std::env::var(k).ok()
            }) {
                Ok(t) => t,
                // An unconfigured forge without a token is one the user does
                // not use; only a configured one is worth a complaint.
                Err(e) => {
                    if explicit {
                        problems.push(e.to_string());
                    }
                    continue;
                }
            };
            let forge: Arc<dyn Forge> = match kind {
                Kind::GitHub => {
                    let scope = f.scope.clone().unwrap_or_else(|| "involves:@me".into());
                    Arc::new(pullpen_forge::github::GitHub::new(
                        f.host(),
                        token,
                        agents.clone(),
                        &scope,
                        cfg.merge.delete_branch,
                    ))
                }
                Kind::GitLab => Arc::new(pullpen_forge::gitlab::GitLab::new(
                    f.host(),
                    token,
                    agents.clone(),
                    f.groups.clone(),
                    cfg.merge.delete_branch,
                )),
            };
            kinds.push(kind);
            forges.push(Entry { forge, me: OnceCell::new() });
        }
        let (tx, rx) = channel();
        let live = Live {
            rt,
            forges: Arc::new(forges),
            tx,
            rx,
            wake: sections.iter().map(|_| Arc::new(Notify::new())).collect(),
            last: Arc::default(),
            repos: Arc::new(cfg.repos.clone()),
            worktree: cfg.checkout.worktree,
        };
        for (i, s) in sections.iter().enumerate() {
            live.spawn_section(i, s.clone(), cfg.sync.every(), cache_dir.clone(), since);
        }
        Ok((live, kinds, problems))
    }

    fn spawn_section(&self, i: usize, section: Section, every: Duration, cache_dir: PathBuf, since: Timestamp) {
        let (forges, tx, wake, last) = (self.forges.clone(), self.tx.clone(), self.wake[i].clone(), self.last.clone());
        self.rt.spawn(async move {
            let q = Query::parse(&section.query);
            let mut backoff = 0u64;
            loop {
                let (mut prs, mut total, mut failed, mut caps) = (Vec::new(), 0, None, HashMap::new());
                for e in forges.iter().filter(|e| q.reaches(e.forge.kind())) {
                    let kind = e.forge.kind();
                    let page = match e.me.get_or_try_init(|| e.forge.viewer()).await {
                        Ok(me) => e.forge.search(&q, me, LIMIT).await,
                        Err(err) => Err(err.clone()),
                    };
                    match page {
                        Ok(p) => {
                            total += p.total;
                            prs.extend(p.prs);
                            // The receiver is gone only once the app has quit.
                            let _ = tx.send(match p.hidden {
                                Some(why) => Msg::Down(kind, why),
                                None => Msg::Up(kind),
                            });
                        }
                        Err(err) => failed = Some((kind, e.forge.host().to_string(), err)),
                    }
                    caps.extend(e.forge.capabilities());
                }
                let client_side = q.terms.iter().any(|t| t.client_side());
                let armed_view = section.view == View::Armed;
                prs.retain(|p| {
                    let fresh = matches!(p.state, PrState::Merged { at, .. } if at >= since);
                    let caps = caps.get(&format!("{}/{}", p.id.host, p.id.repo)).cloned().unwrap_or_default();
                    q.keeps(p, readiness(p, &caps)) || (armed_view && fresh)
                });
                if client_side {
                    total = prs.len();
                }
                sort(&mut prs, section.sort);
                let _ = tx.send(Msg::Caps(caps));
                let wait = match failed {
                    None => {
                        backoff = 0;
                        // A cache that will not write costs only a slower
                        // first paint next time.
                        let _ = cache::store(&cache_dir, &section, &prs, total);
                        if let Ok(mut l) = last.lock() {
                            *l = Some(Instant::now());
                        }
                        let _ = tx.send(Msg::Synced { section: i, prs, total });
                        every
                    }
                    Some((kind, host, err)) => {
                        backoff = if err.transient() { (backoff * 2).clamp(1, BACKOFF_MAX) } else { every.as_secs() };
                        let _ = tx.send(Msg::Down(kind, describe(&host, &err, backoff)));
                        Duration::from_secs(backoff)
                    }
                };
                tokio::select! {
                    _ = tokio::time::sleep(wait) => {}
                    _ = wake.notified() => {}
                }
            }
        });
    }
}

/// The status line's words for a background failure.
fn describe(host: &str, e: &Error, retry: u64) -> String {
    match e {
        Error::Server { status, .. } => format!("{host} {status} · retrying in {retry}s"),
        Error::RateLimited { retry_in } => format!("{host} rate limited · retrying in {retry_in}s"),
        Error::Network(_) => format!("{host} unreachable · retrying in {retry}s"),
        Error::Auth(_) => format!("not signed in to {host}"),
        Error::Refused(why) | Error::Decode(why) => format!("{host}: {why}"),
    }
}

fn sort(prs: &mut [Pr], by: Sort) {
    match by {
        Sort::Updated => prs.sort_by_key(|p| std::cmp::Reverse(p.updated_at)),
        Sort::Created => prs.sort_by_key(|p| std::cmp::Reverse(p.created_at)),
        Sort::Age => prs.sort_by_key(|a| a.created_at),
        Sort::Repo => prs.sort_by(|a, b| (&a.id.repo, a.id.number).cmp(&(&b.id.repo, b.id.number))),
    }
}

impl Backend for Live {
    fn run(&mut self, job: Job) {
        let (forges, tx, wake) = (self.forges.clone(), self.tx.clone(), self.wake.clone());
        let (repos, worktree) = (self.repos.clone(), self.worktree);
        self.rt.spawn(async move {
            let f = forges.iter().find(|e| e.forge.kind() == job.target.forge && e.forge.host() == job.target.host);
            let (outcome, note) = match (&job.action, f) {
                (Action::Checkout { branch }, _) => {
                    let (id, branch) = (job.target.clone(), branch.clone());
                    let cwd = std::env::current_dir().unwrap_or_default();
                    let r =
                        tokio::task::spawn_blocking(move || checkout::checkout(&id, &branch, &repos, worktree, &cwd))
                            .await;
                    match r.unwrap_or_else(|e| Err(e.to_string())) {
                        Ok(wher) => (Outcome::Done, Some(wher)),
                        Err(reason) => (Outcome::Failed { reason }, None),
                    }
                }
                (_, None) => (Outcome::Failed { reason: format!("not signed in to {}", job.target.host) }, None),
                (_, Some(e)) => match pullpen_forge::run(e.forge.as_ref(), &job).await {
                    Ok(note) => (Outcome::Done, note),
                    Err(err) => (Outcome::Failed { reason: err.to_string() }, None),
                },
            };
            if outcome == Outcome::Done {
                // After an action, every section may have changed (PLAN.md §4.3).
                wake.iter().for_each(|w| w.notify_one());
            }
            let _ = tx.send(Msg::Job(JobResult { job, outcome, note }));
        });
    }

    fn fetch(&mut self, id: PrId, diff: bool) {
        let (forges, tx) = (self.forges.clone(), self.tx.clone());
        self.rt.spawn(async move {
            let Some(e) = forges.iter().find(|e| e.forge.kind() == id.forge && e.forge.host() == id.host) else {
                return;
            };
            let msg = if diff {
                match e.forge.diff(&id).await {
                    Ok(files) => Msg::Diff(id, files),
                    Err(err) => Msg::Unfetched { id, diff, why: err.to_string() },
                }
            } else {
                match e.forge.detail(&id).await {
                    Ok(d) => Msg::Detail(id, d),
                    Err(err) => Msg::Unfetched { id, diff, why: err.to_string() },
                }
            };
            let _ = tx.send(msg);
        });
    }

    fn refresh(&mut self, section: Option<usize>) {
        match section {
            Some(i) => self.wake.get(i).iter().for_each(|w| w.notify_one()),
            None => self.wake.iter().for_each(|w| w.notify_one()),
        }
    }

    fn messages(&mut self) -> Vec<Msg> {
        self.rx.try_iter().collect()
    }

    fn sync_label(&self) -> String {
        match self.last.lock().ok().and_then(|l| *l) {
            None => "syncing…".into(),
            Some(t) => {
                let s = t.elapsed().as_secs();
                if s < 60 { format!("synced {s}s") } else { format!("synced {}m", s / 60) }
            }
        }
    }

    fn open(&mut self, url: &str) -> Result<(), String> {
        open::that_detached(url).map_err(|e| format!("opening {url}: {e}"))
    }
}

#[cfg(test)]
mod tests;
