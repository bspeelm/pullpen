//! GitHub: GraphQL to read, REST to write (PLAN.md §4.1).

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use pullpen_core::diff::FileDiff;
use pullpen_core::query::{Is, Query, Term};
use pullpen_core::time::{Timestamp, duration, parse_rfc3339, rfc3339};
use pullpen_core::*;
use serde_json::{Value, json};

use crate::http::{self, graphql, send};
use crate::{Agents, Error, Forge as ForgeTrait, Page, Result, failed_branches, set_branch};

const SEARCH: &str = include_str!("search.graphql");
/// Mergeability and check details, asked for separately and a few PRs at a
/// time: computing mergeability is what makes a search of 30 PRs slow enough
/// for GitHub to time it out with a 502.
const DETAILS: &str = include_str!("checks.graphql");
const DETAIL_BATCH: usize = 10;
const DETAIL: &str = include_str!("detail.graphql");
const RUNS: &str = include_str!("runs.graphql");

/// What a branch's rulesets require. Rulesets are readable with read
/// access, unlike classic branch protection, which only admins can read.
#[derive(Clone, Debug, Default)]
struct Rules {
    reviews: Option<u32>,
    checks: Vec<String>,
    strict: bool,
    methods: Option<Vec<MergeMethod>>,
}

pub struct GitHub {
    http: reqwest::Client,
    host: String,
    api: String,
    token: String,
    agents: Agents,
    /// Appended to a query that names no user, repo or org, so it does not
    /// search every PR on GitHub (PLAN.md §6.1).
    scope: String,
    delete_branch: bool,
    rules: Mutex<HashMap<(String, String), Rules>>,
    caps: Mutex<HashMap<String, Capabilities>>,
}

impl GitHub {
    pub fn new(host: &str, token: String, agents: Agents, scope: &str, delete_branch: bool) -> Self {
        let api =
            if host == "github.com" { "https://api.github.com".to_string() } else { format!("https://{host}/api/v3") };
        Self::with_api(host, &api, token, agents, scope, delete_branch)
    }

    /// For tests, and for anything that serves the API elsewhere.
    pub fn with_api(host: &str, api: &str, token: String, agents: Agents, scope: &str, delete_branch: bool) -> Self {
        GitHub {
            http: http::client(),
            host: host.into(),
            api: api.trim_end_matches('/').into(),
            token,
            agents,
            scope: scope.into(),
            delete_branch,
            rules: Mutex::default(),
            caps: Mutex::default(),
        }
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.req_as(method, path, "application/vnd.github+json")
    }

    /// `accept` replaces the JSON default rather than adding to it: a second
    /// Accept header leaves the answer's format to the server's choosing.
    fn req_as(&self, method: reqwest::Method, path: &str, accept: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.api))
            .bearer_auth(&self.token)
            .header("Accept", accept)
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    fn graphql_url(&self) -> String {
        if self.api.ends_with("/api/v3") {
            self.api.replace("/api/v3", "/api/graphql")
        } else {
            format!("{}/graphql", self.api)
        }
    }

    async fn gql(&self, query: &str, vars: Value) -> Result<Value> {
        let req = self.http.post(self.graphql_url()).bearer_auth(&self.token);
        graphql(&self.host, req, query, vars).await
    }

    async fn rules(&self, repo: &str, base: &str) -> Rules {
        let key = (repo.to_string(), base.to_string());
        if let Some(r) = self.rules.lock().ok().and_then(|m| m.get(&key).cloned()) {
            return r;
        }
        let path = format!("/repos/{repo}/rules/branches/{base}");
        // A repo without rulesets, or one the token cannot read rules on, is
        // a repo with no known requirements, not an error.
        let list = send(&self.host, self.req(reqwest::Method::GET, &path)).await.unwrap_or(Value::Null);
        let rules = parse_rules(&list);
        if let Ok(mut m) = self.rules.lock() {
            m.insert(key, rules.clone());
        }
        rules
    }

    fn server_query(&self, q: &Query) -> String {
        let mut parts = vec!["is:pr".to_string()];
        for t in q.terms.iter().filter(|t| !t.client_side()) {
            parts.push(match t {
                Term::Is(Is::Open) => "is:open".into(),
                Term::Is(Is::Merged) => "is:merged".into(),
                Term::Is(Is::Closed) => "is:closed".into(),
                Term::Is(Is::Draft) => "draft:true".into(),
                Term::Author(a) => format!("author:{a}"),
                Term::ReviewRequested(a) => format!("review-requested:{a}"),
                Term::ReviewedBy(a) => format!("reviewed-by:{a}"),
                Term::Repo(r) => format!("repo:{r}"),
                Term::Org(o) | Term::Group(o) => format!("org:{o}"),
                Term::Label(l) => format!("label:\"{l}\""),
                Term::Base(b) => format!("base:{b}"),
                Term::MergedWithin(s) => format!("merged:>={}", rfc3339(Timestamp(Timestamp::now().0 - s))),
                Term::Text(t) => t.clone(),
                _ => continue,
            });
        }
        if !q.narrowed() && !self.scope.is_empty() {
            parts.push(self.scope.clone());
        }
        // Search's default order is relevance; every section wants recency.
        parts.push("sort:updated-desc".into());
        parts.join(" ")
    }

    /// The checks on the head of each base branch a merge failed on. Without
    /// them the failure still shows, as needing attention, so a failure here
    /// is logged rather than failing the section.
    async fn branch_heads(&self, prs: &mut [Pr]) {
        for chunk in failed_branches(prs).chunks(20) {
            let (mut decl, mut body, mut vars) = (Vec::new(), String::new(), serde_json::Map::new());
            for (i, (repo, base)) in chunk.iter().enumerate() {
                let (owner, name) = repo.split_once('/').unwrap_or((repo, ""));
                decl.push(format!("$o{i}: String!, $n{i}: String!, $q{i}: String!"));
                body += &format!(
                    " b{i}: repository(owner: $o{i}, name: $n{i}) {{ ref(qualifiedName: $q{i}) {{ target {{ ... on Commit {{ statusCheckRollup {{ state }} }} }} }} }}"
                );
                vars.insert(format!("o{i}"), json!(owner));
                vars.insert(format!("n{i}"), json!(name));
                vars.insert(format!("q{i}"), json!(format!("refs/heads/{base}")));
            }
            let d = match self.gql(&format!("query({}) {{{body} }}", decl.join(", ")), Value::Object(vars)).await {
                Ok(d) => d,
                Err(e) => return tracing::warn!(host = %self.host, error = %e, "base branch checks"),
            };
            for (i, key) in chunk.iter().enumerate() {
                if let Some(st) =
                    d.pointer(&format!("/b{i}/ref/target/statusCheckRollup/state")).and_then(Value::as_str)
                {
                    set_branch(prs, key, rollup(st));
                }
            }
        }
    }

    async fn node_id(&self, id: &PrId) -> Result<String> {
        let (owner, name) = id.repo.split_once('/').ok_or_else(|| Error::Decode(format!("repo {}", id.repo)))?;
        let q = "query($o:String!,$n:String!,$num:Int!){repository(owner:$o,name:$n){pullRequest(number:$num){id}}}";
        let d = self.gql(q, json!({"o": owner, "n": name, "num": id.number})).await?;
        d.pointer("/repository/pullRequest/id")
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or_else(|| Error::Refused(format!("no PR #{} in {}", id.number, id.repo)))
    }
}

/// The status line's words for rows the forge withheld. No count: the line
/// is shared by every section, and a section that searches closed PRs too
/// would report a number no list shows. An org that refuses classic tokens is
/// common enough, and its fix specific enough, to name.
fn hidden_reason(why: &str) -> String {
    match why.split('`').nth(1).filter(|_| why.contains("personal access token (classic)")) {
        Some(org) => format!("{org} hides its PRs from classic tokens; run `pullpen setup` and use your gh login"),
        None => format!("some PRs hidden: {why}"),
    }
}

fn parse_rules(list: &Value) -> Rules {
    let mut r = Rules::default();
    for rule in list.as_array().into_iter().flatten() {
        let p = &rule["parameters"];
        match rule["type"].as_str() {
            Some("pull_request") => {
                let n = p["required_approving_review_count"].as_u64().unwrap_or(0) as u32;
                r.reviews = Some(r.reviews.map_or(n, |m| m.max(n)));
                if let Some(ms) = p["allowed_merge_methods"].as_array() {
                    r.methods = Some(ms.iter().filter_map(|m| method(m.as_str()?)).collect());
                }
            }
            Some("required_status_checks") => {
                r.strict |= p["strict_required_status_checks_policy"].as_bool().unwrap_or(false);
                r.checks.extend(
                    p["required_status_checks"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|c| c["context"].as_str().map(String::from)),
                );
            }
            _ => {}
        }
    }
    r
}

fn method(s: &str) -> Option<MergeMethod> {
    match s.to_ascii_lowercase().as_str() {
        "squash" => Some(MergeMethod::Squash),
        "merge" => Some(MergeMethod::Merge),
        "rebase" => Some(MergeMethod::Rebase),
        _ => None,
    }
}

fn s<'a>(v: &'a Value, p: &str) -> &'a str {
    v.pointer(p).and_then(Value::as_str).unwrap_or("")
}

/// One check run or status context, as a state.
fn check_state(n: &Value) -> CheckState {
    match (n["__typename"] == "CheckRun", s(n, "/status"), s(n, "/conclusion"), s(n, "/state")) {
        (true, "COMPLETED", "SUCCESS" | "NEUTRAL" | "SKIPPED", _) | (false, _, _, "SUCCESS") => CheckState::Passed,
        (true, "COMPLETED", _, _) | (false, _, _, "FAILURE" | "ERROR") => CheckState::Failed,
        (true, "QUEUED", _, _) => CheckState::Queued,
        _ => CheckState::Running,
    }
}

/// A status-check rollup's state as a pipeline's.
fn rollup(state: &str) -> PipelineState {
    match state {
        "SUCCESS" => PipelineState::Passed,
        "FAILURE" | "ERROR" => PipelineState::Failed,
        _ => PipelineState::Running,
    }
}

/// The merge commit's checks, which on GitHub are its pipeline: the
/// workflows a push to the base branch starts, deploys among them.
fn pipeline(commit: &Value, url: String) -> Option<Pipeline> {
    let rollup = commit.pointer("/statusCheckRollup").filter(|r| !r.is_null())?;
    let nodes = rollup.pointer("/contexts/nodes").and_then(Value::as_array).cloned().unwrap_or_default();
    let named = |want: CheckState| {
        nodes
            .iter()
            .find(|n| check_state(n) == want)
            .map(|n| if n["__typename"] == "CheckRun" { s(n, "/name") } else { s(n, "/context") }.to_string())
    };
    let state = self::rollup(s(rollup, "/state"));
    let job = match state {
        PipelineState::Failed => named(CheckState::Failed),
        PipelineState::Running => named(CheckState::Running).or_else(|| named(CheckState::Queued)),
        PipelineState::Passed => None,
    };
    Some(Pipeline {
        state,
        passed: nodes.iter().filter(|n| check_state(n) == CheckState::Passed).count() as u32,
        total: nodes.len() as u32,
        job: job.unwrap_or_default(),
        url,
        branch: None,
    })
}

/// A PR's runs from `runs.graphql`: the merge commit's first, then the
/// head's. A commit's runs across every workflow and app count as one run,
/// since that is what decides whether it can merge or has deployed.
fn runs(pr: &Value) -> Vec<Run> {
    let web = s(pr, "/url").rsplit_once("/pull/").map_or("", |(w, _)| w);
    let commits =
        [(Which::AfterMerge, pr.pointer("/mergeCommit")), (Which::BeforeMerge, pr.pointer("/commits/nodes/0/commit"))];
    commits.into_iter().filter_map(|(which, c)| run(which, c.filter(|c| !c.is_null())?, web)).collect()
}

fn run(which: Which, c: &Value, web: &str) -> Option<Run> {
    let opt = |v: &Value, p: &str| parse_rfc3339(s(v, p));
    let mut jobs = Vec::new();
    let mut event = String::new();
    for suite in c.pointer("/checkSuites/nodes").and_then(Value::as_array).into_iter().flatten() {
        let wf = &suite["workflowRun"];
        // CodeQL's default setup says `dynamic`; the PR's own workflows
        // say what started them.
        if event.is_empty() || event == "dynamic" {
            event = s(wf, "/event").replace('_', " ");
        }
        let group = if wf.is_null() { s(suite, "/app/name") } else { s(wf, "/workflow/name") };
        for n in suite.pointer("/checkRuns/nodes").and_then(Value::as_array).into_iter().flatten() {
            let steps = n.pointer("/steps/nodes").and_then(Value::as_array).into_iter().flatten();
            jobs.push(JobRun {
                id: n["databaseId"].as_i64().map_or(String::new(), |i| i.to_string()),
                name: s(n, "/name").into(),
                group: group.into(),
                state: step_state(n),
                started: opt(n, "/startedAt"),
                finished: opt(n, "/completedAt"),
                steps: steps
                    .map(|st| Step {
                        name: s(st, "/name").into(),
                        state: step_state(st),
                        started: opt(st, "/startedAt"),
                        finished: opt(st, "/completedAt"),
                    })
                    .collect(),
                url: s(n, "/detailsUrl").into(),
                log: !wf.is_null(),
            });
        }
    }
    for n in c.pointer("/status/contexts").and_then(Value::as_array).into_iter().flatten() {
        jobs.push(JobRun {
            id: String::new(),
            name: s(n, "/context").into(),
            group: "status".into(),
            state: check_state(n),
            started: opt(n, "/createdAt"),
            finished: None,
            steps: vec![],
            url: s(n, "/targetUrl").into(),
            log: false,
        });
    }
    if jobs.is_empty() {
        return None;
    }
    let state = overall(&jobs);
    Some(Run {
        which,
        sha: s(c, "/oid").chars().take(7).collect(),
        event,
        started: jobs.iter().filter_map(|j| j.started).min(),
        finished: if matches!(state, CheckState::Passed | CheckState::Failed) {
            jobs.iter().filter_map(|j| j.finished).max()
        } else {
            None
        },
        state,
        url: format!("{web}/commit/{}/checks", s(c, "/oid")),
        jobs,
    })
}

/// A check run's or a step's state, which share GitHub's status and
/// conclusion fields.
fn step_state(n: &Value) -> CheckState {
    match (s(n, "/status"), s(n, "/conclusion")) {
        ("COMPLETED", "SUCCESS" | "NEUTRAL" | "SKIPPED") => CheckState::Passed,
        ("COMPLETED", _) => CheckState::Failed,
        ("QUEUED" | "PENDING" | "WAITING" | "REQUESTED", _) => CheckState::Queued,
        _ => CheckState::Running,
    }
}

fn time(v: &Value, p: &str) -> Timestamp {
    parse_rfc3339(s(v, p)).unwrap_or_default()
}

fn checks(rollup: &Value, required: &[String]) -> Checks {
    let mut c = Checks::default();
    let mut seen = Vec::new();
    for n in rollup.pointer("/contexts/nodes").and_then(Value::as_array).into_iter().flatten() {
        let (name, state) = if n["__typename"] == "CheckRun" {
            let st = match (s(n, "/status"), s(n, "/conclusion")) {
                ("COMPLETED", "SUCCESS" | "NEUTRAL" | "SKIPPED") => CheckState::Passed,
                ("COMPLETED", _) => CheckState::Failed,
                _ => CheckState::Running,
            };
            (s(n, "/name"), st)
        } else {
            let st = match s(n, "/state") {
                "SUCCESS" => CheckState::Passed,
                "FAILURE" | "ERROR" => CheckState::Failed,
                _ => CheckState::Running,
            };
            (s(n, "/context"), st)
        };
        // With no ruleset naming required checks, every check counts: the
        // forge would not merge past a red one either way.
        if !required.is_empty() && !required.iter().any(|r| r == name) {
            continue;
        }
        c.required_total += 1;
        match state {
            CheckState::Passed => c.required_passed += 1,
            CheckState::Failed => c.failed.push(CheckRun { name: name.into(), detail: String::new() }),
            _ => c.running += 1,
        }
        seen.push(name.to_string());
    }
    // A required check that has not reported is "Expected": pending, not
    // absent. Unless the list was cut short, in which case it may simply be
    // past the cut, and the rollup's verdict covers it.
    let truncated = rollup.pointer("/contexts/pageInfo/hasNextPage") == Some(&Value::Bool(true));
    for _ in required.iter().filter(|r| !seen.contains(r)) {
        c.required_total += 1;
        match (truncated, s(rollup, "/state")) {
            (true, "SUCCESS") => c.required_passed += 1,
            _ => c.running += 1,
        }
    }
    c
}

impl GitHub {
    fn to_pr(&self, n: &Value, rules: &Rules) -> Pr {
        let repo = s(n, "/repository/nameWithOwner").to_string();
        let r = &n["repository"];
        let mut allowed: Vec<MergeMethod> = [
            (MergeMethod::Squash, "squashMergeAllowed"),
            (MergeMethod::Merge, "mergeCommitAllowed"),
            (MergeMethod::Rebase, "rebaseMergeAllowed"),
        ]
        .into_iter()
        .filter(|(_, k)| r[*k].as_bool().unwrap_or(true))
        .map(|(m, _)| m)
        .collect();
        if let Some(ms) = &rules.methods {
            allowed.retain(|m| ms.contains(m));
        }
        let default = method(s(r, "/viewerDefaultMergeMethod"))
            .filter(|m| allowed.contains(m))
            .or(allowed.first().copied())
            .unwrap_or(MergeMethod::Merge);
        let approvals: Vec<&Value> =
            n.pointer("/latestOpinionatedReviews/nodes").and_then(Value::as_array).into_iter().flatten().collect();
        let approved_by: Vec<String> =
            approvals.iter().filter(|a| a["state"] == "APPROVED").map(|a| s(a, "/author/login").to_string()).collect();
        let have = approved_by.len() as u32;
        let decision = s(n, "/reviewDecision");
        let need = rules.reviews.unwrap_or(match decision {
            "REVIEW_REQUIRED" => have + 1,
            "APPROVED" => have,
            _ => 0,
        });
        let pending = n
            .pointer("/reviewRequests/nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|r| {
                let rr = &r["requestedReviewer"];
                if rr["__typename"] == "Team" {
                    format!("{}/{}", s(rr, "/organization/login"), s(rr, "/slug"))
                } else {
                    s(rr, "/login").to_string()
                }
            })
            .filter(|l| !l.is_empty())
            .collect();
        let behind = s(n, "/mergeStateStatus") == "BEHIND";
        let mergeable = match s(n, "/mergeable") {
            "CONFLICTING" => Mergeable::Conflict,
            "MERGEABLE" if behind => Mergeable::Behind,
            "MERGEABLE" => Mergeable::Clean,
            _ => Mergeable::Unknown,
        };
        if let Ok(mut c) = self.caps.lock() {
            let e = c.entry(format!("{}/{repo}", self.host)).or_default();
            e.allowed_methods = allowed.clone();
            // GitHub reports BEHIND only when protection requires the branch
            // to be current, so seeing it is as good as reading the rule.
            e.requires_up_to_date |= rules.strict || behind;
        }
        let login = s(n, "/author/login");
        let am = &n["autoMergeRequest"];
        Pr {
            id: PrId { forge: Forge::GitHub, host: self.host.clone(), repo, number: n["number"].as_u64().unwrap_or(0) },
            title: s(n, "/title").into(),
            author: Author {
                login: login.into(),
                is_agent: self.agents.is_agent(login, s(n, "/author/__typename") == "Bot"),
            },
            head: s(n, "/headRefName").into(),
            base: s(n, "/baseRefName").into(),
            additions: n["additions"].as_u64().unwrap_or(0) as u32,
            deletions: n["deletions"].as_u64().unwrap_or(0) as u32,
            files: n["changedFiles"].as_u64().unwrap_or(0) as u32,
            draft: n["isDraft"].as_bool().unwrap_or(false),
            checks: n
                .pointer("/commits/nodes/0/commit/statusCheckRollup")
                .map_or_else(Checks::default, |r| checks(r, &rules.checks)),
            reviews: Reviews { have, need, approved_by, changes_requested: decision == "CHANGES_REQUESTED", pending },
            mergeable,
            // GitHub keeps the request on a PR after it merges or closes.
            auto_merge: (!am.is_null() && s(n, "/state") == "OPEN").then(|| AutoMerge {
                method: method(s(am, "/mergeMethod")).unwrap_or(default),
                armed_by: s(am, "/enabledBy/login").into(),
                armed_at: time(am, "/enabledAt"),
            }),
            merge_train: None,
            state: match s(n, "/state") {
                "MERGED" => PrState::Merged { at: time(n, "/mergedAt"), method: default },
                "CLOSED" => PrState::Closed,
                _ => PrState::Open,
            },
            updated_at: time(n, "/updatedAt"),
            created_at: time(n, "/createdAt"),
            url: s(n, "/url").into(),
            repo_default_method: default,
            repo_allowed_methods: allowed,
            post_merge: n.pointer("/mergeCommit").filter(|c| !c.is_null()).and_then(|c| {
                let web = s(n, "/url").rsplit_once("/pull/").map_or("", |(w, _)| w);
                pipeline(c, format!("{web}/commit/{}", s(c, "/oid")))
            }),
        }
    }
}

impl GitHub {
    fn to_detail(&self, p: &Value) -> PrDetail {
        let mut checks = Vec::new();
        let (mut first, mut last) = (i64::MAX, 0);
        for n in p
            .pointer("/commits/nodes/0/commit/statusCheckRollup/contexts/nodes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let run = n["__typename"] == "CheckRun";
            let state = check_state(n);
            let (from, to) = (time(n, "/startedAt"), time(n, "/completedAt"));
            if from.0 > 0 {
                first = first.min(from.0);
                last = last.max(to.0);
            }
            checks.push(CheckLine {
                name: if run { s(n, "/name") } else { s(n, "/context") }.into(),
                state,
                required: n["isRequired"] == true,
                duration: if from.0 > 0 && to.0 > 0 { duration(from, to) } else { String::new() },
            });
        }
        let t = &p["timelineItems"];
        let mut items: Vec<Activity> = Vec::new();
        for n in t["nodes"].as_array().into_iter().flatten() {
            let who = |p: &str| (s(n, p).to_string(), self.agents.is_agent(s(n, p), false));
            let act = |(who, is_agent): (String, bool), kind, at: &str, body: &str| Activity {
                who,
                is_agent,
                kind,
                at: time(n, at),
                body: body.into(),
            };
            let a = match s(n, "/__typename") {
                "PullRequestReview" => {
                    let kind = match s(n, "/state") {
                        "APPROVED" => ActivityKind::Approved,
                        "CHANGES_REQUESTED" => ActivityKind::ChangesRequested,
                        _ => ActivityKind::Commented,
                    };
                    act(who("/author/login"), kind, "/createdAt", s(n, "/body"))
                }
                "PullRequestCommit" => {
                    let login = if s(n, "/commit/author/user/login").is_empty() {
                        "/commit/author/name"
                    } else {
                        "/commit/author/user/login"
                    };
                    act(
                        who(login),
                        ActivityKind::Pushed { commits: 1 },
                        "/commit/committedDate",
                        s(n, "/commit/messageHeadline"),
                    )
                }
                "AutoMergeEnabledEvent" => act(who("/actor/login"), ActivityKind::ArmedAutoMerge, "/createdAt", ""),
                "AutoMergeDisabledEvent" => act(who("/actor/login"), ActivityKind::DisarmedAutoMerge, "/createdAt", ""),
                "ReviewRequestedEvent" => {
                    let target = if s(n, "/requestedReviewer/login").is_empty() {
                        "/requestedReviewer/slug"
                    } else {
                        "/requestedReviewer/login"
                    };
                    act(
                        who(target),
                        ActivityKind::ReviewRequested { by: Some(s(n, "/actor/login").into()) },
                        "/createdAt",
                        "",
                    )
                }
                "IssueComment" => act(who("/author/login"), ActivityKind::Commented, "/createdAt", s(n, "/body")),
                _ => continue,
            };
            // Consecutive pushes by one person read as one push of several.
            match (items.last_mut(), &a.kind) {
                (Some(prev), ActivityKind::Pushed { .. })
                    if prev.who == a.who && matches!(prev.kind, ActivityKind::Pushed { .. }) =>
                {
                    if let ActivityKind::Pushed { commits } = &mut prev.kind {
                        *commits += 1;
                    }
                    prev.at = a.at;
                    prev.body.clear();
                }
                _ => items.push(a),
            }
        }
        items.reverse();
        let conversation =
            items.iter().filter(|a| !a.body.is_empty() || a.kind == ActivityKind::Approved).take(8).cloned().collect();
        PrDetail {
            description: s(p, "/body").replace("\r\n", "\n"),
            checks,
            checks_took: if last > first { duration(Timestamp(first), Timestamp(last)) } else { String::new() },
            activity: items.into_iter().take(6).collect(),
            conversation,
            conversation_total: t["totalCount"].as_u64().unwrap_or(0) as usize,
        }
    }
}

#[async_trait]
impl ForgeTrait for GitHub {
    fn kind(&self) -> Forge {
        Forge::GitHub
    }

    fn host(&self) -> &str {
        &self.host
    }

    async fn viewer(&self) -> Result<String> {
        let d = self.gql("query{viewer{login}}", json!({})).await?;
        Ok(s(&d, "/viewer/login").to_string())
    }

    async fn places(&self) -> Result<Vec<crate::Place>> {
        let d = self.gql("query{viewer{login organizations(first:100){nodes{login}}}}", json!({})).await?;
        let logins = std::iter::once(s(&d, "/viewer/login")).chain(
            d.pointer("/viewer/organizations/nodes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|o| s(o, "/login")),
        );
        Ok(logins.map(|l| crate::Place { path: l.into(), name: l.into() }).collect())
    }

    async fn search(&self, q: &Query, _me: &str, limit: usize) -> Result<Page> {
        let text = self.server_query(q);
        let (mut nodes, mut total, mut after, mut why) = (Vec::new(), 0, Value::Null, None);
        while nodes.len() < limit {
            let req = self.http.post(self.graphql_url()).bearer_auth(&self.token);
            let (d, w) = http::graphql_partial(&self.host, req, SEARCH, json!({ "q": text, "after": after })).await?;
            why = why.or(w);
            total = d.pointer("/search/issueCount").and_then(Value::as_u64).unwrap_or(0) as usize;
            nodes.extend(d.pointer("/search/nodes").and_then(Value::as_array).cloned().unwrap_or_default());
            match d.pointer("/search/pageInfo") {
                Some(p) if p["hasNextPage"] == true => after = p["endCursor"].clone(),
                _ => break,
            }
        }
        // A row the token may not read comes back null, counted but empty.
        let withheld = nodes.iter().filter(|n| n.is_null()).count();
        nodes.retain(|n| n.get("number").is_some());
        nodes.truncate(limit);
        let hidden = (withheld > 0).then(|| hidden_reason(why.as_deref().unwrap_or("no reason given")));
        for batch in nodes.chunks_mut(DETAIL_BATCH) {
            let ids: Vec<&str> = batch.iter().map(|n| s(n, "/id")).collect();
            let d = self.gql(DETAILS, json!({ "ids": ids })).await?;
            for (n, extra) in batch.iter_mut().zip(d["nodes"].as_array().into_iter().flatten()) {
                for k in ["mergeable", "mergeStateStatus", "commits"] {
                    n[k] = extra[k].clone();
                }
            }
        }
        let mut prs = Vec::with_capacity(nodes.len());
        for n in &nodes {
            let rules = self.rules(s(n, "/repository/nameWithOwner"), s(n, "/baseRefName")).await;
            prs.push(self.to_pr(n, &rules));
        }
        self.branch_heads(&mut prs).await;
        Ok(Page { prs, total: total.saturating_sub(withheld), hidden })
    }

    fn capabilities(&self) -> HashMap<String, Capabilities> {
        self.caps.lock().map(|c| c.clone()).unwrap_or_default()
    }

    async fn merge(&self, id: &PrId, m: MergeMethod) -> Result<bool> {
        let path = format!("/repos/{}/pulls/{}/merge", id.repo, id.number);
        send(&self.host, self.req(reqwest::Method::PUT, &path).json(&json!({ "merge_method": m.word() }))).await?;
        if self.delete_branch {
            let pr =
                send(&self.host, self.req(reqwest::Method::GET, &format!("/repos/{}/pulls/{}", id.repo, id.number)))
                    .await?;
            // Only a branch in the same repo is ours to delete; a fork's is not.
            if s(&pr, "/head/repo/full_name") == id.repo {
                let path = format!("/repos/{}/git/refs/heads/{}", id.repo, s(&pr, "/head/ref"));
                // The merge has happened; a branch that will not delete is
                // reported as kept, not as a failed merge.
                return Ok(send(&self.host, self.req(reqwest::Method::DELETE, &path)).await.is_ok());
            }
        }
        Ok(false)
    }

    async fn arm(&self, id: &PrId, m: MergeMethod) -> Result<()> {
        let node = self.node_id(id).await?;
        let q = "mutation($id:ID!,$m:PullRequestMergeMethod!){enablePullRequestAutoMerge(input:{pullRequestId:$id,mergeMethod:$m}){clientMutationId}}";
        self.gql(q, json!({ "id": node, "m": m.word().to_uppercase() })).await.map(|_| ())
    }

    async fn disarm(&self, id: &PrId) -> Result<()> {
        let node = self.node_id(id).await?;
        let q = "mutation($id:ID!){disablePullRequestAutoMerge(input:{pullRequestId:$id}){clientMutationId}}";
        self.gql(q, json!({ "id": node })).await.map(|_| ())
    }

    async fn approve(&self, id: &PrId) -> Result<()> {
        let path = format!("/repos/{}/pulls/{}/reviews", id.repo, id.number);
        send(&self.host, self.req(reqwest::Method::POST, &path).json(&json!({ "event": "APPROVE" }))).await.map(|_| ())
    }

    async fn comment(&self, id: &PrId, body: &str) -> Result<()> {
        let path = format!("/repos/{}/issues/{}/comments", id.repo, id.number);
        send(&self.host, self.req(reqwest::Method::POST, &path).json(&json!({ "body": body }))).await.map(|_| ())
    }

    async fn close(&self, id: &PrId) -> Result<()> {
        let path = format!("/repos/{}/pulls/{}", id.repo, id.number);
        send(&self.host, self.req(reqwest::Method::PATCH, &path).json(&json!({ "state": "closed" }))).await.map(|_| ())
    }

    async fn update(&self, id: &PrId) -> Result<()> {
        let path = format!("/repos/{}/pulls/{}/update-branch", id.repo, id.number);
        send(&self.host, self.req(reqwest::Method::PUT, &path).json(&json!({}))).await.map(|_| ())
    }

    async fn detail(&self, id: &PrId) -> Result<PrDetail> {
        let (owner, name) = id.repo.split_once('/').ok_or_else(|| Error::Decode(format!("repo {}", id.repo)))?;
        let d = self.gql(DETAIL, json!({ "o": owner, "n": name, "num": id.number })).await?;
        Ok(self.to_detail(&d["repository"]["pullRequest"]))
    }

    async fn runs(&self, id: &PrId) -> Result<Vec<Run>> {
        let (owner, name) = id.repo.split_once('/').ok_or_else(|| Error::Decode(format!("repo {}", id.repo)))?;
        let d = self.gql(RUNS, json!({ "o": owner, "n": name, "num": id.number })).await?;
        Ok(runs(&d["repository"]["pullRequest"]))
    }

    /// GitHub serves a job's log only once the job finishes; a running job
    /// answers 404 (ADR-020).
    async fn log(&self, id: &PrId, job: &str) -> Result<String> {
        let path = format!("/repos/{}/actions/jobs/{job}/logs", id.repo);
        http::text(&self.host, self.req(reqwest::Method::GET, &path)).await
    }

    async fn diff(&self, id: &PrId) -> Result<Vec<FileDiff>> {
        let path = format!("/repos/{}/pulls/{}", id.repo, id.number);
        let body = send(&self.host, self.req_as(reqwest::Method::GET, &path, "application/vnd.github.diff")).await?;
        Ok(pullpen_core::diff::parse(body.as_str().unwrap_or("")))
    }
}

#[cfg(test)]
mod tests;
