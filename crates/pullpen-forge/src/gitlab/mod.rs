//! GitLab: GraphQL to read, REST to write (ADR-008). A merge request is a
//! PR here, and `!812` is shown as `#812`.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;
use pullpen_core::diff::FileDiff;
use pullpen_core::query::{Is, Query, Term};
use pullpen_core::time::{Timestamp, duration, parse_rfc3339, rfc3339};
use pullpen_core::*;
use serde_json::{Map, Value, json};

use crate::http::{self, encode_path, graphql, send};
use crate::{Agents, Error, Forge as ForgeTrait, Page, Result, failed_branches, set_branch};

const FRAGMENT: &str = include_str!("mr.graphql");
const DETAIL: &str = include_str!("detail.graphql");
const RUNS: &str = include_str!("runs.graphql");
const LANDED: &str = include_str!("landed.graphql");
/// A running job's log is asked for every few seconds; its tail is enough.
const LOG_TAIL: usize = 256 * 1024;

pub struct GitLab {
    http: reqwest::Client,
    host: String,
    base: String,
    token: String,
    agents: Agents,
    /// Groups searched when a query names no project, group or person.
    groups: Vec<String>,
    delete_branch: bool,
    caps: Mutex<HashMap<String, Capabilities>>,
}

/// Where one search runs: a project, a group, or the instance-wide list,
/// which GitLab only allows with a person filter.
enum Root {
    Project(String),
    Group(String),
    Instance,
}

impl GitLab {
    pub fn new(host: &str, token: String, agents: Agents, groups: Vec<String>, delete_branch: bool) -> Self {
        Self::with_base(host, &format!("https://{host}"), token, agents, groups, delete_branch)
    }

    pub fn with_base(
        host: &str,
        base: &str,
        token: String,
        agents: Agents,
        groups: Vec<String>,
        delete_branch: bool,
    ) -> Self {
        GitLab {
            http: http::client(),
            host: host.into(),
            base: base.trim_end_matches('/').into(),
            token,
            agents,
            groups,
            delete_branch,
            caps: Mutex::default(),
        }
    }

    async fn gql(&self, query: &str, vars: Value) -> Result<Value> {
        let req = self.http.post(format!("{}/api/graphql", self.base)).bearer_auth(&self.token);
        graphql(&self.host, req, query, vars).await
    }

    fn rest(&self, method: reqwest::Method, id: &PrId, tail: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/api/v4/projects/{}/merge_requests/{}{tail}", self.base, encode_path(&id.repo), id.number);
        self.http.request(method, url).bearer_auth(&self.token)
    }

    /// The searches a query needs, each with its filter arguments.
    fn plan(&self, q: &Query, me: &str) -> Vec<(Root, Map<String, Value>)> {
        let who = |a: &str| if a == "@me" { me.to_string() } else { a.trim_start_matches('@').to_string() };
        let mut args = Map::new();
        let (mut projects, mut groups, mut people) = (Vec::new(), Vec::new(), false);
        let mut state = "opened";
        let mut text = Vec::new();
        for t in q.terms.iter().filter(|t| !t.client_side()) {
            match t {
                Term::Is(Is::Merged) => state = "merged",
                Term::Is(Is::Closed) => state = "closed",
                Term::Is(Is::Draft) => {
                    args.insert("draft".into(), json!(true));
                }
                Term::Author(a) => {
                    args.insert("authorUsername".into(), json!(who(a)));
                    people = true;
                }
                Term::ReviewRequested(a) => {
                    args.insert("reviewerUsername".into(), json!(who(a)));
                    people = true;
                }
                Term::ReviewedBy(a) => {
                    args.insert("approvedBy".into(), json!([who(a)]));
                    people = true;
                }
                Term::Repo(r) => projects.push(r.clone()),
                Term::Org(g) | Term::Group(g) => groups.push(g.clone()),
                Term::Label(l) => {
                    if let Some(a) = args.entry("labels").or_insert(json!([])).as_array_mut() {
                        a.push(json!(l))
                    }
                }
                Term::Base(b) => {
                    args.insert("targetBranches".into(), json!([b]));
                }
                Term::MergedWithin(secs) => {
                    args.insert("mergedAfter".into(), json!(rfc3339(Timestamp(Timestamp::now().0 - secs))));
                }
                Term::Text(t) => text.push(t.clone()),
                _ => {}
            }
        }
        args.insert("state".into(), json!(state));
        let mut with_search = args.clone();
        if !text.is_empty() {
            with_search.insert("search".into(), json!(text.join(" ")));
        }
        if !projects.is_empty() {
            return projects.into_iter().map(|p| (Root::Project(p), with_search.clone())).collect();
        }
        if groups.is_empty() && !people {
            groups = self.groups.clone();
        }
        if !groups.is_empty() {
            return groups.into_iter().map(|g| (Root::Group(g), with_search.clone())).collect();
        }
        if people {
            return vec![(Root::Instance, args)];
        }
        // Nothing narrows the query and no groups are configured: the PRs that
        // involve you, which is what an unscoped list can sensibly mean.
        ["authorUsername", "reviewerUsername", "assigneeUsername"]
            .into_iter()
            .map(|k| {
                let mut a = args.clone();
                a.insert(k.into(), json!(me));
                (Root::Instance, a)
            })
            .collect()
    }

    async fn run_root(&self, root: &Root, args: &Map<String, Value>, limit: usize) -> Result<(Vec<Value>, usize)> {
        let decl: String = args.keys().map(|k| format!(", ${k}: {}", arg_type(k))).collect();
        let pass: String = args.keys().map(|k| format!(", {k}: ${k}")).collect();
        let conn = format!(
            "mergeRequests(first: 50, after: $after, sort: UPDATED_DESC{pass}) {{ count pageInfo {{ hasNextPage endCursor }} nodes {{ ...mr }} }}"
        );
        let (query, path) = match root {
            Root::Project(_) => (
                format!(
                    "{FRAGMENT}query($path: ID!, $after: String{decl}) {{ x: project(fullPath: $path) {{ {conn} }} }}"
                ),
                "/x/mergeRequests",
            ),
            Root::Group(_) => (
                format!(
                    "{FRAGMENT}query($path: ID!, $after: String{decl}) {{ x: group(fullPath: $path) {{ {} }} }}",
                    conn.replacen("sort: UPDATED_DESC", "sort: UPDATED_DESC, includeSubgroups: true", 1)
                ),
                "/x/mergeRequests",
            ),
            Root::Instance => (
                format!(
                    "{FRAGMENT}query($after: String{decl}) {{ x: mergeRequests(first: 50, after: $after, sort: UPDATED_DESC{pass}) {{ count pageInfo {{ hasNextPage endCursor }} nodes {{ ...mr }} }} }}"
                ),
                "/x",
            ),
        };
        let mut vars = Value::Object(args.clone());
        if let Root::Project(p) | Root::Group(p) = root {
            vars["path"] = json!(p);
        }
        let mut nodes = Vec::new();
        let total = loop {
            let d = self.gql(&query, vars.clone()).await?;
            let c =
                d.pointer(path).filter(|v| !v.is_null()).ok_or_else(|| Error::Refused(missing(root, &self.host)))?;
            nodes.extend(c["nodes"].as_array().cloned().unwrap_or_default());
            match &c["pageInfo"] {
                p if p["hasNextPage"] == true && nodes.len() < limit => vars["after"] = p["endCursor"].clone(),
                _ => break c["count"].as_u64().unwrap_or(0) as usize,
            }
        };
        Ok((nodes, total))
    }

    fn to_pr(&self, n: &Value) -> Pr {
        let repo = s(n, "/project/fullPath").to_string();
        let ff = n.pointer("/project/mergeRequestsFfOnlyEnabled").and_then(Value::as_bool).unwrap_or(false);
        let squash = n["squashOnMerge"].as_bool().unwrap_or(false);
        // A fast-forward-only project merges one way; otherwise the choice
        // is between a merge commit and squashing it.
        let allowed = if ff { vec![MergeMethod::Rebase] } else { vec![MergeMethod::Squash, MergeMethod::Merge] };
        let default = if ff {
            MergeMethod::Rebase
        } else if squash {
            MergeMethod::Squash
        } else {
            MergeMethod::Merge
        };
        let checks = match s(n, "/headPipeline/status") {
            "" => Checks::default(),
            "SUCCESS" | "SKIPPED" | "MANUAL" => Checks { required_total: 1, required_passed: 1, ..Default::default() },
            "FAILED" | "CANCELED" => Checks {
                required_total: 1,
                failed: vec![CheckRun { name: "pipeline".into(), detail: s(n, "/headPipeline/status").to_lowercase() }],
                ..Default::default()
            },
            _ => Checks { required_total: 1, running: 1, ..Default::default() },
        };
        let approved: Vec<String> = names(n, "/approvedBy/nodes");
        let need = n["approvalsRequired"].as_u64().unwrap_or(0) as u32;
        let left = n["approvalsLeft"].as_u64().unwrap_or(0) as u32;
        let have = need.saturating_sub(left).max(if need == 0 { approved.len() as u32 } else { 0 });
        let pending = names(n, "/reviewers/nodes").into_iter().filter(|r| !approved.contains(r)).collect();
        let approved_by = approved.clone();
        let status = s(n, "/detailedMergeStatus");
        // GitLab can report detailedMergeStatus CONFLICT for a merge request
        // its own page calls ready to merge, with `conflicts` false: the status
        // lags a fresh check, and the flag does not. So only the flag decides a
        // conflict; the status still says when the branch is behind or GitLab
        // is mid-check. UNCHECKED is not mid-check: every open MR goes back to
        // it when its target branch moves, and stays there until something asks,
        // which a merge does; GitLab then refuses with the reason if it must.
        let mergeable = match status {
            _ if n["conflicts"] == true => Mergeable::Conflict,
            "NEED_REBASE" => Mergeable::Behind,
            "CHECKING" | "PREPARING" | "APPROVALS_SYNCING" => Mergeable::Unknown,
            _ => Mergeable::Clean,
        };
        if let Ok(mut c) = self.caps.lock() {
            let e = c.entry(format!("{}/{repo}", self.host)).or_default();
            e.allowed_methods = allowed.clone();
            e.requires_up_to_date |= ff || status == "NEED_REBASE";
            e.merge_trains |= n["mergeTrainsCount"].as_u64().is_some_and(|c| c > 0);
        }
        let login = s(n, "/author/username");
        let at = parse_rfc3339(s(n, "/updatedAt")).unwrap_or_default();
        Pr {
            id: PrId { forge: Forge::GitLab, host: self.host.clone(), repo, number: s(n, "/iid").parse().unwrap_or(0) },
            title: s(n, "/title").into(),
            author: Author {
                login: login.into(),
                is_agent: self.agents.is_agent(login, n.pointer("/author/bot") == Some(&json!(true))),
            },
            head: s(n, "/sourceBranch").into(),
            base: s(n, "/targetBranch").into(),
            additions: n.pointer("/diffStatsSummary/additions").and_then(Value::as_u64).unwrap_or(0) as u32,
            deletions: n.pointer("/diffStatsSummary/deletions").and_then(Value::as_u64).unwrap_or(0) as u32,
            files: n.pointer("/diffStatsSummary/fileCount").and_then(Value::as_u64).unwrap_or(0) as u32,
            draft: n["draft"].as_bool().unwrap_or(false),
            checks,
            reviews: Reviews { have, need, approved_by, changes_requested: status == "REQUESTED_CHANGES", pending },
            mergeable,
            auto_merge: (n["autoMergeEnabled"] == true).then(|| AutoMerge {
                method: if squash { MergeMethod::Squash } else { default },
                armed_by: s(n, "/mergeUser/username").into(),
                armed_at: at,
            }),
            merge_train: n["mergeTrainIndex"].as_u64().map(|i| MergeTrainPos {
                position: i as u32 + 1,
                length: n["mergeTrainsCount"].as_u64().unwrap_or(0) as u32,
            }),
            state: match s(n, "/state") {
                "merged" => PrState::Merged { at: parse_rfc3339(s(n, "/mergedAt")).unwrap_or(at), method: default },
                "closed" => PrState::Closed,
                _ => PrState::Open,
            },
            updated_at: at,
            created_at: parse_rfc3339(s(n, "/createdAt")).unwrap_or_default(),
            url: s(n, "/webUrl").into(),
            repo_default_method: default,
            repo_allowed_methods: allowed,
            post_merge: None,
        }
    }

    /// Each merged MR's pipeline on its merge commit and target branch.
    /// GitLab's `ref` filter also matches merge-request pipelines, any of
    /// which can share the SHA, so only pushes count, and the answer must
    /// name the branch itself (`on_branch`). Asked a few at a time: GitLab
    /// refuses a query over 250 complexity, which twenty MRs exceed. Without
    /// it a row still shows, as merged with no pipeline, so a failure is
    /// logged rather than failing the section.
    async fn post_merge(&self, prs: &mut [Pr], landed: &[(PrId, String)]) {
        let landed: Vec<(&PrId, &String, String)> = landed
            .iter()
            .filter_map(|(id, sha)| prs.iter().find(|p| &p.id == id).map(|p| (id, sha, p.base.clone())))
            .collect();
        for chunk in landed.chunks(8) {
            let (mut decl, mut body, mut vars) = (Vec::new(), String::new(), Map::new());
            for (i, (id, sha, base)) in chunk.iter().enumerate() {
                decl.push(format!("$r{i}: ID!, $s{i}: String!, $b{i}: String!"));
                body += &format!(
                    " p{i}: project(fullPath: $r{i}) {{ pipelines(sha: $s{i}, ref: $b{i}, source: \"push\", first: 1) {{ nodes {{ ref status path jobs(first: 50) {{ nodes {{ name status }} }} }} }} }}"
                );
                vars.insert(format!("r{i}"), json!(id.repo));
                vars.insert(format!("s{i}"), json!(sha));
                vars.insert(format!("b{i}"), json!(base));
            }
            let query = format!("query({}) {{{body} }}", decl.join(", "));
            let d = match self.gql(&query, Value::Object(vars)).await {
                Ok(d) => d,
                Err(e) => return tracing::warn!(host = %self.host, error = %e, "post-merge pipelines"),
            };
            for (i, (id, _, base)) in chunk.iter().enumerate() {
                let Some(pl) = on_branch(&d, i, base) else { continue };
                if let Some(p) = prs.iter_mut().find(|p| &p.id == *id) {
                    p.post_merge = Some(pipeline(pl, &self.base));
                }
            }
        }
    }

    /// The newest pipeline on each target branch a merge failed on, logged
    /// rather than failing the section when it cannot be read. Only pushes,
    /// for the reason `post_merge` gives.
    async fn branch_heads(&self, prs: &mut [Pr]) {
        for chunk in failed_branches(prs).chunks(20) {
            let (mut decl, mut body, mut vars) = (Vec::new(), String::new(), Map::new());
            for (i, (repo, base)) in chunk.iter().enumerate() {
                decl.push(format!("$r{i}: ID!, $b{i}: String!"));
                body += &format!(
                    " p{i}: project(fullPath: $r{i}) {{ pipelines(ref: $b{i}, source: \"push\", first: 1) {{ nodes {{ ref status }} }} }}"
                );
                vars.insert(format!("r{i}"), json!(repo));
                vars.insert(format!("b{i}"), json!(base));
            }
            let d = match self.gql(&format!("query({}) {{{body} }}", decl.join(", ")), Value::Object(vars)).await {
                Ok(d) => d,
                Err(e) => return tracing::warn!(host = %self.host, error = %e, "target branch pipelines"),
            };
            for (i, key) in chunk.iter().enumerate() {
                if let Some(pl) = on_branch(&d, i, &key.1) {
                    set_branch(prs, key, status(s(pl, "/status")));
                }
            }
        }
    }

    async fn merge_with(&self, id: &PrId, m: MergeMethod, auto: bool) -> Result<()> {
        let mut body = json!({ "squash": m == MergeMethod::Squash, "should_remove_source_branch": self.delete_branch });
        if auto {
            // `auto_merge` is GitLab 16.5's name; older instances know only
            // the deprecated one, and newer ones still accept it.
            body["auto_merge"] = json!(true);
            body["merge_when_pipeline_succeeds"] = json!(true);
        }
        send(&self.host, self.rest(reqwest::Method::PUT, id, "/merge").json(&body)).await.map(|_| ())
    }
}

/// A pipeline status as pullpen counts it: see `pipeline`.
fn status(s: &str) -> PipelineState {
    match s {
        "SUCCESS" | "SKIPPED" => PipelineState::Passed,
        "FAILED" | "CANCELED" => PipelineState::Failed,
        _ => PipelineState::Running,
    }
}

/// A GitLab pipeline as pullpen reports one. A canceled pipeline deployed
/// nothing, so it counts as failed; one stopped at a manual job is still
/// running, since what it would deploy has not been.
fn pipeline(p: &Value, base: &str) -> Pipeline {
    let jobs = p.pointer("/jobs/nodes").and_then(Value::as_array).cloned().unwrap_or_default();
    let first = |st: &[&str]| jobs.iter().find(|j| st.contains(&s(j, "/status"))).map(|j| s(j, "/name").to_string());
    let state = status(s(p, "/status"));
    let job = match (state, s(p, "/status")) {
        (PipelineState::Passed, _) => None,
        (PipelineState::Failed, _) => first(&["FAILED", "CANCELED"]),
        (_, "MANUAL") => first(&["MANUAL"]).map(|j| format!("{j} (manual)")),
        _ => first(&["RUNNING", "PENDING", "CREATED"]),
    };
    Pipeline {
        state,
        passed: jobs.iter().filter(|j| s(j, "/status") == "SUCCESS").count() as u32,
        total: jobs.len() as u32,
        job: job.unwrap_or_default(),
        url: format!("{base}{}", s(p, "/path")),
        branch: None,
    }
}

/// The `p{i}` pipeline of an aliased query, if it ran for `branch` itself.
fn on_branch<'a>(d: &'a Value, i: usize, branch: &str) -> Option<&'a Value> {
    d.pointer(&format!("/p{i}/pipelines/nodes/0")).filter(|p| s(p, "/ref") == branch)
}

/// A pipeline as the pipeline screen shows it. A pipeline stopped at a
/// manual job is still running, as `pipeline` above says.
fn run(which: Which, p: &Value, base: &str) -> Run {
    let opt = |v: &Value, k: &str| parse_rfc3339(s(v, k));
    let jobs: Vec<JobRun> = p
        .pointer("/jobs/nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|j| JobRun {
            id: s(j, "/id").rsplit('/').next().unwrap_or_default().into(),
            name: s(j, "/name").into(),
            group: s(j, "/stage/name").into(),
            state: match s(j, "/status") {
                "SUCCESS" | "SKIPPED" => CheckState::Passed,
                "FAILED" | "CANCELED" => CheckState::Failed,
                "RUNNING" => CheckState::Running,
                _ => CheckState::Queued,
            },
            started: opt(j, "/startedAt"),
            finished: opt(j, "/finishedAt"),
            steps: vec![],
            url: format!("{base}{}", s(j, "/webPath")),
            log: true,
        })
        .collect();
    let state = match status(s(p, "/status")) {
        PipelineState::Passed => CheckState::Passed,
        PipelineState::Failed => CheckState::Failed,
        PipelineState::Running => CheckState::Running,
    };
    Run {
        which,
        sha: s(p, "/sha").chars().take(7).collect(),
        event: match s(p, "/source") {
            "merge_request_event" => "merge request".into(),
            other => other.replace('_', " "),
        },
        state,
        started: opt(p, "/startedAt"),
        finished: opt(p, "/finishedAt"),
        url: format!("{base}{}", s(p, "/path")),
        jobs,
    }
}

/// What to say when a group or project is not there, which GitLab reports
/// by answering with nothing at all.
fn missing(root: &Root, host: &str) -> String {
    match root {
        Root::Group(g) => {
            format!(
                "no group {g} on {host}, or no access; a group is a path like my-org, not a username (gitlab.groups)"
            )
        }
        Root::Project(p) => format!("no project {p} on {host}, or no access"),
        Root::Instance => format!("{host} answered with nothing"),
    }
}

fn arg_type(k: &str) -> &'static str {
    match k {
        "state" => "MergeRequestState",
        "draft" => "Boolean",
        "labels" | "targetBranches" | "approvedBy" => "[String!]",
        "mergedAfter" => "Time",
        _ => "String",
    }
}

fn s<'a>(v: &'a Value, p: &str) -> &'a str {
    v.pointer(p).and_then(Value::as_str).unwrap_or("")
}

fn names(v: &Value, p: &str) -> Vec<String> {
    v.pointer(p)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|n| n["username"].as_str().map(String::from))
        .collect()
}

#[async_trait]
impl ForgeTrait for GitLab {
    fn kind(&self) -> Forge {
        Forge::GitLab
    }

    fn host(&self) -> &str {
        &self.host
    }

    async fn viewer(&self) -> Result<String> {
        let d = self.gql("query{currentUser{username}}", json!({})).await?;
        Ok(s(&d, "/currentUser/username").to_string())
    }

    async fn places(&self) -> Result<Vec<crate::Place>> {
        let d = self.gql("query{currentUser{groups(first:100){nodes{fullPath name}}}}", json!({})).await?;
        let groups = d.pointer("/currentUser/groups/nodes").and_then(Value::as_array).into_iter().flatten();
        Ok(groups.map(|g| crate::Place { path: s(g, "/fullPath").into(), name: s(g, "/name").into() }).collect())
    }

    async fn search(&self, q: &Query, me: &str, limit: usize) -> Result<Page> {
        let plan = self.plan(q, me);
        let several = plan.len() > 1;
        let text: Vec<String> =
            q.terms.iter().filter_map(|t| if let Term::Text(t) = t { Some(t.to_lowercase()) } else { None }).collect();
        let (mut seen, mut prs, mut total) = (HashSet::new(), Vec::new(), 0);
        let mut missing_roots = Vec::new();
        let mut last_err: Option<Error> = None;
        let mut landed = Vec::new();
        for (root, args) in &plan {
            // One group that does not exist should not hide the others.
            let (nodes, count) = match self.run_root(root, args, limit).await {
                Ok(r) => r,
                Err(Error::Refused(why)) => {
                    missing_roots.push(why.clone());
                    last_err = Some(Error::Refused(why));
                    continue;
                }
                Err(e) => return Err(e),
            };
            total += count;
            for n in &nodes {
                let pr = self.to_pr(n);
                // A fast-forward merge makes no merge commit: the head is
                // what landed on the base branch.
                let sha = [s(n, "/mergeCommitSha"), s(n, "/diffHeadSha")].into_iter().find(|x| !x.is_empty());
                if let (true, Some(sha)) = (pr.merged(), sha) {
                    landed.push((pr.id.clone(), sha.to_string()));
                }
                // The instance-wide list has no text search; match titles here.
                let text_ok =
                    !matches!(root, Root::Instance) || text.iter().all(|t| pr.title.to_lowercase().contains(t));
                if text_ok && seen.insert(pr.id.clone()) {
                    prs.push(pr);
                }
            }
        }
        prs.sort_by_key(|p| std::cmp::Reverse(p.updated_at));
        prs.truncate(limit);
        self.post_merge(&mut prs, &landed).await;
        self.branch_heads(&mut prs).await;
        // A union of several searches overlaps, so its counts would add up
        // to more PRs than exist; the deduplicated rows are the honest total.
        if missing_roots.len() == plan.len() && last_err.is_some() {
            return Err(Error::Refused(missing_roots.join("; ")));
        }
        let hidden = (!missing_roots.is_empty()).then(|| missing_roots.join("; "));
        Ok(Page { total: if several { prs.len() } else { total }, prs, hidden })
    }

    fn capabilities(&self) -> HashMap<String, Capabilities> {
        self.caps.lock().map(|c| c.clone()).unwrap_or_default()
    }

    async fn merge(&self, id: &PrId, m: MergeMethod) -> Result<bool> {
        // GitLab deletes the branch itself when asked, as part of the merge.
        self.merge_with(id, m, false).await.map(|()| self.delete_branch)
    }

    async fn arm(&self, id: &PrId, m: MergeMethod) -> Result<()> {
        self.merge_with(id, m, true).await
    }

    async fn disarm(&self, id: &PrId) -> Result<()> {
        send(&self.host, self.rest(reqwest::Method::POST, id, "/cancel_merge_when_pipeline_succeeds")).await.map(|_| ())
    }

    async fn approve(&self, id: &PrId) -> Result<()> {
        send(&self.host, self.rest(reqwest::Method::POST, id, "/approve")).await.map(|_| ())
    }

    async fn comment(&self, id: &PrId, body: &str) -> Result<()> {
        send(&self.host, self.rest(reqwest::Method::POST, id, "/notes").json(&json!({ "body": body })))
            .await
            .map(|_| ())
    }

    async fn close(&self, id: &PrId) -> Result<()> {
        send(&self.host, self.rest(reqwest::Method::PUT, id, "").json(&json!({ "state_event": "close" })))
            .await
            .map(|_| ())
    }

    async fn update(&self, id: &PrId) -> Result<()> {
        send(&self.host, self.rest(reqwest::Method::PUT, id, "/rebase")).await.map(|_| ())
    }

    async fn runs(&self, id: &PrId) -> Result<Vec<Run>> {
        let d = self.gql(RUNS, json!({ "path": id.repo, "iid": id.number.to_string() })).await?;
        let m = &d["project"]["mergeRequest"];
        let mut out = Vec::new();
        // After a fast-forward the merge commit is the head, as in `post_merge`.
        let sha = [s(m, "/mergeCommitSha"), s(m, "/diffHeadSha")].into_iter().find(|x| !x.is_empty());
        if let (Some(sha), "merged") = (sha, s(m, "/state")) {
            let vars = json!({ "path": id.repo, "sha": sha, "ref": s(m, "/targetBranch") });
            let d = self.gql(LANDED, vars).await?;
            if let Some(p) = d.pointer("/project/pipelines/nodes/0").filter(|p| s(p, "/ref") == s(m, "/targetBranch")) {
                out.push(run(Which::AfterMerge, p, &self.base));
            }
        }
        if let Some(p) = m.get("headPipeline").filter(|p| !p.is_null()) {
            out.push(run(Which::BeforeMerge, p, &self.base));
        }
        Ok(out)
    }

    async fn log(&self, id: &PrId, job: &str) -> Result<String> {
        let url = format!("{}/api/v4/projects/{}/jobs/{job}/trace", self.base, encode_path(&id.repo));
        let req = self.http.get(url).bearer_auth(&self.token).header("Range", format!("bytes=-{LOG_TAIL}"));
        http::text(&self.host, req).await
    }

    async fn detail(&self, id: &PrId) -> Result<PrDetail> {
        let d = self.gql(DETAIL, json!({ "path": id.repo, "iid": id.number.to_string() })).await?;
        let m = &d["project"]["mergeRequest"];
        let pipe = &m["headPipeline"];
        let checks = pipe
            .pointer("/jobs/nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|j| CheckLine {
                name: s(j, "/name").into(),
                state: match s(j, "/status") {
                    "SUCCESS" | "SKIPPED" | "MANUAL" => CheckState::Passed,
                    "FAILED" | "CANCELED" => CheckState::Failed,
                    "CREATED" | "PENDING" | "WAITING_FOR_RESOURCE" | "SCHEDULED" => CheckState::Queued,
                    _ => CheckState::Running,
                },
                required: true,
                duration: j["duration"].as_i64().map_or(String::new(), |d| duration(Timestamp(0), Timestamp(d))),
            })
            .collect();
        let took = match (parse_rfc3339(s(pipe, "/startedAt")), parse_rfc3339(s(pipe, "/finishedAt"))) {
            (Some(a), Some(b)) => duration(a, b),
            _ => String::new(),
        };
        let mut items: Vec<Activity> = m
            .pointer("/notes/nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|n| {
                let body = s(n, "/body");
                // GitLab's system notes are sentences; these are the ones the
                // timeline shows, recognised by how they start.
                let kind = match n["system"] == true {
                    false => ActivityKind::Commented,
                    true if body.starts_with("approved") => ActivityKind::Approved,
                    true if body.starts_with("requested changes") => ActivityKind::ChangesRequested,
                    true if body.contains("automatic merge") || body.contains("auto-merge") => {
                        if body.starts_with("cancel") {
                            ActivityKind::DisarmedAutoMerge
                        } else {
                            ActivityKind::ArmedAutoMerge
                        }
                    }
                    true if body.starts_with("requested review") => ActivityKind::ReviewRequested { by: None },
                    true if body.starts_with("added ") && body.contains("commit") => ActivityKind::Pushed {
                        commits: body.split_whitespace().nth(1).and_then(|n| n.parse().ok()).unwrap_or(1),
                    },
                    true => return None,
                };
                let login = s(n, "/author/username");
                let text = if kind == ActivityKind::Commented { body } else { "" };
                Some(Activity {
                    who: login.into(),
                    is_agent: self.agents.is_agent(login, n.pointer("/author/bot") == Some(&json!(true))),
                    kind,
                    at: parse_rfc3339(s(n, "/createdAt")).unwrap_or_default(),
                    body: text.into(),
                })
            })
            .collect();
        items.sort_by_key(|a| std::cmp::Reverse(a.at));
        let conversation: Vec<Activity> =
            items.iter().filter(|a| !a.body.is_empty() || a.kind == ActivityKind::Approved).take(8).cloned().collect();
        Ok(PrDetail {
            description: s(m, "/description").replace("\r\n", "\n"),
            checks,
            checks_took: took,
            conversation_total: conversation.len(),
            conversation,
            activity: items.into_iter().take(6).collect(),
        })
    }

    async fn diff(&self, id: &PrId) -> Result<Vec<FileDiff>> {
        let mut text = String::new();
        for page in 1..=10 {
            let list =
                send(&self.host, self.rest(reqwest::Method::GET, id, &format!("/diffs?per_page=100&page={page}")))
                    .await?;
            let files = list.as_array().cloned().unwrap_or_default();
            for f in &files {
                let (a, b) = (s(f, "/old_path"), s(f, "/new_path"));
                text.push_str(&format!("diff --git a/{a} b/{b}\n--- a/{a}\n+++ b/{b}\n{}\n", s(f, "/diff")));
            }
            if files.len() < 100 {
                break;
            }
        }
        Ok(pullpen_core::diff::parse(&text))
    }
}

#[cfg(test)]
mod tests;
