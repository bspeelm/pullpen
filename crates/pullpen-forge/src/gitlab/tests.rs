use pullpen_core::query::Query;
use serde_json::json;
use wiremock::matchers::{body_partial_json, body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn gl(server: &MockServer, groups: &[&str]) -> GitLab {
    let groups = groups.iter().map(|g| g.to_string()).collect();
    GitLab::with_base("gitlab.com", &server.uri(), "t0ken".into(), Agents(Agents::default_logins()), groups, true)
}

fn node() -> Value {
    json!({
        "iid": "812", "title": "Bump go toolchain", "webUrl": "https://gitlab.com/platform/runner/-/merge_requests/812",
        "draft": false, "createdAt": "2026-10-01T00:00:00Z", "updatedAt": "2026-10-02T00:00:00Z",
        "sourceBranch": "renovate/go", "targetBranch": "main", "state": "opened", "mergedAt": null,
        "author": { "username": "renovate-bot", "bot": true },
        "diffStatsSummary": { "additions": 6, "deletions": 6, "fileCount": 2 },
        "headPipeline": { "status": "RUNNING" },
        "approvalsRequired": 2, "approvalsLeft": 1,
        "approvedBy": { "nodes": [ { "username": "lena" } ] },
        "reviewers": { "nodes": [ { "username": "lena" }, { "username": "ash" } ] },
        "conflicts": false, "detailedMergeStatus": "CI_STILL_RUNNING", "autoMergeEnabled": true,
        "mergeUser": { "username": "ash" }, "mergeTrainIndex": 1, "mergeTrainsCount": 3, "squashOnMerge": true,
        "project": { "fullPath": "platform/runner", "mergeRequestsFfOnlyEnabled": false, "squashReadOnly": false }
    })
}

#[tokio::test]
async fn search_maps_a_merge_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/graphql"))
        .and(body_string_contains("group(fullPath: $path)"))
        .and(body_partial_json(json!({ "variables": { "path": "platform", "state": "opened" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "x": { "mergeRequests": {
            "count": 17, "pageInfo": { "hasNextPage": false, "endCursor": null }, "nodes": [node()]
        } } } })))
        .mount(&server)
        .await;
    let page = gl(&server, &["platform"]).search(&Query::parse("is:open"), "me", 50).await.unwrap();
    assert_eq!(page.total, 17);
    let p = &page.prs[0];
    assert_eq!((p.id.repo.as_str(), p.id.number, p.id.forge), ("platform/runner", 812, Forge::GitLab));
    assert!(p.author.is_agent);
    assert_eq!((p.additions, p.deletions, p.files), (6, 6, 2));
    assert_eq!((p.checks.running, p.checks.required_total), (1, 1));
    assert_eq!((p.reviews.have, p.reviews.need), (1, 2));
    assert_eq!(p.reviews.pending, vec!["ash".to_string()], "approvers are not pending");
    assert_eq!(p.merge_train, Some(MergeTrainPos { position: 2, length: 3 }));
    assert_eq!(p.auto_merge.as_ref().map(|a| (a.method, a.armed_by.as_str())), Some((MergeMethod::Squash, "ash")));
    assert_eq!(p.repo_default_method, MergeMethod::Squash);
}

#[test]
fn queries_pick_the_narrowest_root() {
    let g = GitLab::with_base("gitlab.com", "http://x", "t".into(), Agents::default(), vec![], false);
    let roots = |q: &str| -> Vec<String> {
        g.plan(&Query::parse(q), "me")
            .iter()
            .map(|(r, a)| {
                let who = ["authorUsername", "reviewerUsername", "assigneeUsername"]
                    .iter()
                    .find(|k| a.contains_key(**k))
                    .copied()
                    .unwrap_or("-");
                match r {
                    Root::Project(p) => format!("project:{p}"),
                    Root::Group(g) => format!("group:{g}"),
                    Root::Instance => format!("instance:{who}"),
                }
            })
            .collect()
    };
    assert_eq!(roots("is:open repo:platform/runner"), ["project:platform/runner"]);
    assert_eq!(roots("is:open group:platform"), ["group:platform"]);
    assert_eq!(roots("is:open review-requested:@me"), ["instance:reviewerUsername"]);
    assert_eq!(
        roots("is:armed"),
        ["instance:authorUsername", "instance:reviewerUsername", "instance:assigneeUsername"]
    );
}

fn id() -> PrId {
    PrId { forge: Forge::GitLab, host: "gitlab.com".into(), repo: "platform/runner".into(), number: 812 }
}

#[tokio::test]
async fn writes_go_to_rest_with_the_project_path_encoded() {
    let server = MockServer::start().await;
    let base = "/api/v4/projects/platform%2Frunner/merge_requests/812";
    for (m, p, body) in [
        (
            "PUT",
            format!("{base}/merge"),
            json!({ "squash": true, "should_remove_source_branch": true, "auto_merge": true }),
        ),
        ("POST", format!("{base}/cancel_merge_when_pipeline_succeeds"), json!(null)),
        ("POST", format!("{base}/approve"), json!(null)),
        ("POST", format!("{base}/notes"), json!({ "body": "lgtm" })),
        ("PUT", base.to_string(), json!({ "state_event": "close" })),
    ] {
        let mut mock = Mock::given(method(m)).and(path(p));
        if !body.is_null() {
            mock = mock.and(body_partial_json(body));
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({}))).expect(1).mount(&server).await;
    }
    let g = gl(&server, &[]);
    g.arm(&id(), MergeMethod::Squash).await.unwrap();
    g.disarm(&id()).await.unwrap();
    g.approve(&id()).await.unwrap();
    g.comment(&id(), "lgtm").await.unwrap();
    g.close(&id()).await.unwrap();
}

#[tokio::test]
async fn refusals_carry_gitlabs_message_in_either_shape() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/platform%2Frunner/merge_requests/812/merge"))
        .respond_with(ResponseTemplate::new(405).set_body_json(json!({ "message": "405 Method Not Allowed" })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/platform%2Frunner/merge_requests/812/notes"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({ "message": { "body": ["can't be blank"] } })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/platform%2Frunner/merge_requests/812/approve"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({ "message": "401 Unauthorized" })))
        .mount(&server)
        .await;
    let g = gl(&server, &[]);
    assert_eq!(g.merge(&id(), MergeMethod::Merge).await.unwrap_err(), Error::Refused("405 Method Not Allowed".into()));
    assert_eq!(g.comment(&id(), "").await.unwrap_err(), Error::Refused("body can't be blank".into()));
    assert_eq!(g.approve(&id()).await.unwrap_err(), Error::Auth("gitlab.com".into()));
}

#[tokio::test]
async fn detail_reads_jobs_and_recognises_system_notes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("mergeRequest(iid: $iid)"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "project": { "mergeRequest": {
            "description": "Bump it.",
            "headPipeline": { "startedAt": "2026-10-01T00:00:00Z", "finishedAt": "2026-10-01T00:04:00Z",
                              "jobs": { "nodes": [ { "name": "test", "status": "FAILED", "duration": 38 } ] } },
            "notes": { "nodes": [
                { "system": true, "body": "added 3 commits", "createdAt": "2026-10-01T00:00:00Z", "author": { "username": "renovate-bot", "bot": true } },
                { "system": true, "body": "approved this merge request", "createdAt": "2026-10-01T01:00:00Z", "author": { "username": "lena", "bot": false } },
                { "system": true, "body": "changed the description", "createdAt": "2026-10-01T01:30:00Z", "author": { "username": "lena", "bot": false } },
                { "system": false, "body": "please rebase", "createdAt": "2026-10-01T02:00:00Z", "author": { "username": "ash", "bot": false } }
            ] }
        } } } })))
        .mount(&server)
        .await;
    let d = gl(&server, &[]).detail(&id()).await.unwrap();
    assert_eq!((d.checks[0].state, d.checks[0].duration.as_str()), (CheckState::Failed, "38s"));
    assert_eq!(d.checks_took, "4m 00s");
    let kinds: Vec<ActivityKind> = d.activity.iter().map(|a| a.kind.clone()).collect();
    assert_eq!(
        kinds,
        [ActivityKind::Commented, ActivityKind::Approved, ActivityKind::Pushed { commits: 3 }],
        "unrecognised system notes are skipped"
    );
    assert!(d.activity[2].is_agent);
}

#[tokio::test]
async fn the_diff_is_assembled_from_the_diffs_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/platform%2Frunner/merge_requests/812/diffs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "old_path": "go.mod", "new_path": "go.mod", "diff": "@@ -1 +1 @@\n-go 1.25.2\n+go 1.25.3\n" }
        ])))
        .mount(&server)
        .await;
    let files = gl(&server, &[]).diff(&id()).await.unwrap();
    assert_eq!((files[0].path.as_str(), files[0].additions), ("go.mod", 1));
}

#[tokio::test]
async fn a_group_that_does_not_exist_is_named_and_the_others_still_show() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "variables": { "path": "a5434" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "x": null } })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "variables": { "path": "platform" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "x": { "mergeRequests": {
            "count": 1, "pageInfo": { "hasNextPage": false, "endCursor": null }, "nodes": [node()]
        } } } })))
        .mount(&server)
        .await;
    let page = gl(&server, &["a5434", "platform"]).search(&Query::parse("is:open"), "me", 50).await.unwrap();
    assert_eq!(page.prs.len(), 1, "the group that exists still shows");
    assert!(page.hidden.unwrap().starts_with("no group a5434 on gitlab.com"));
    let alone = gl(&server, &["a5434"]).search(&Query::parse("is:open"), "me", 50).await.err().unwrap();
    assert!(alone.to_string().contains("not a username"), "{alone}");
}

#[tokio::test]
async fn a_conflict_status_without_the_conflicts_flag_is_not_a_conflict() {
    // What gitlab.com sent for a merge request its page called ready to merge.
    let server = MockServer::start().await;
    let mut stale = node();
    stale["conflicts"] = json!(false);
    stale["detailedMergeStatus"] = json!("CONFLICT");
    let mut real = node();
    real["iid"] = json!("813");
    real["conflicts"] = json!(true);
    real["detailedMergeStatus"] = json!("CONFLICT");
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "x": { "mergeRequests": {
            "count": 2, "pageInfo": { "hasNextPage": false, "endCursor": null }, "nodes": [stale, real]
        } } } })))
        .mount(&server)
        .await;
    let page = gl(&server, &["platform"]).search(&Query::parse("is:open"), "me", 50).await.unwrap();
    assert_eq!(page.prs[0].mergeable, Mergeable::Clean, "the status lagged; the flag was right");
    assert_eq!(page.prs[1].mergeable, Mergeable::Conflict);
}

#[tokio::test]
async fn update_rebases_through_the_rebase_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/platform%2Frunner/merge_requests/812/rebase"))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({ "rebase_in_progress": true })))
        .expect(1)
        .mount(&server)
        .await;
    gl(&server, &[]).update(&id()).await.unwrap();
}

#[tokio::test]
async fn a_merged_request_carries_the_pipeline_on_its_merge_commit() {
    let server = MockServer::start().await;
    let mut n = node();
    n["state"] = json!("merged");
    n["mergedAt"] = json!("2026-10-02T00:00:00Z");
    n["mergeCommitSha"] = json!("abc123");
    Mock::given(method("POST"))
        .and(body_string_contains("mergedAfter: $mergedAfter"))
        .and(body_partial_json(json!({ "variables": { "state": "merged" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "x": { "mergeRequests": {
            "count": 1, "pageInfo": { "hasNextPage": false }, "nodes": [n]
        } } } })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "variables": { "r0": "platform/runner", "s0": "abc123", "b0": "main" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "p0": { "pipelines": { "nodes": [ {
            "status": "RUNNING", "path": "/platform/runner/-/pipelines/9",
            "jobs": { "nodes": [ { "name": "build", "status": "SUCCESS" }, { "name": "deploy-staging", "status": "RUNNING" } ] }
        } ] } } } })))
        .expect(1)
        .mount(&server)
        .await;
    let page = gl(&server, &["platform"]).search(&Query::parse("is:merged merged:7d"), "me", 50).await.unwrap();
    let pl = page.prs[0].post_merge.as_ref().unwrap();
    assert_eq!((pl.state, pl.passed, pl.total, pl.job.as_str()), (PipelineState::Running, 1, 2, "deploy-staging"));
    assert_eq!(pl.url, format!("{}/platform/runner/-/pipelines/9", server.uri()));
}

#[test]
fn a_canceled_pipeline_failed_and_a_manual_one_is_still_running() {
    let p = |status: &str, job: &str| {
        pipeline(&json!({ "status": status, "jobs": { "nodes": [ { "name": "deploy", "status": job } ] } }), "")
    };
    assert_eq!(
        (p("CANCELED", "CANCELED").state, p("CANCELED", "CANCELED").job),
        (PipelineState::Failed, "deploy".into())
    );
    assert_eq!(p("MANUAL", "MANUAL").state, PipelineState::Running);
    assert_eq!(p("MANUAL", "MANUAL").job, "deploy (manual)");
}

#[tokio::test]
async fn a_failed_pipeline_asks_whether_the_target_branch_passes_now() {
    let server = MockServer::start().await;
    let mut n = node();
    n["state"] = json!("merged");
    n["mergeCommitSha"] = json!("abc123");
    Mock::given(method("POST"))
        .and(body_string_contains("mergedAfter: $mergedAfter"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "x": { "mergeRequests": {
            "count": 1, "pageInfo": { "hasNextPage": false }, "nodes": [n]
        } } } })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "variables": { "s0": "abc123" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "p0": { "pipelines": { "nodes": [ {
            "status": "FAILED", "path": "/p/-/pipelines/9", "jobs": { "nodes": [ { "name": "deploy", "status": "FAILED" } ] }
        } ] } } } })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("pipelines(ref: $b0, first: 1)"))
        .and(body_partial_json(json!({ "variables": { "r0": "platform/runner", "b0": "main" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "p0": { "pipelines": { "nodes": [
            { "status": "FAILED" } ] } } } })))
        .expect(1)
        .mount(&server)
        .await;
    let page = gl(&server, &["platform"]).search(&Query::parse("is:merged merged:7d"), "me", 50).await.unwrap();
    let pl = page.prs[0].post_merge.as_ref().unwrap();
    assert_eq!((pl.state, pl.branch), (PipelineState::Failed, Some(PipelineState::Failed)), "main is still broken");
}

fn pl(sha: &str, status: &str, source: &str, jobs: Value) -> Value {
    json!({ "sha": sha, "status": status, "source": source, "path": format!("/platform/runner/-/pipelines/{sha}"),
            "startedAt": "2026-10-01T00:00:00Z", "finishedAt": null, "jobs": { "nodes": jobs } })
}

#[tokio::test]
async fn a_merged_mr_has_its_pipeline_on_the_target_branch_then_its_own() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("mergeRequest(iid: $iid)"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "project": { "mergeRequest": {
            "mergeCommitSha": "3f2a91c4e", "diffHeadSha": "9c01e7a2", "targetBranch": "main", "state": "merged",
            "headPipeline": pl("9c01e7a2", "SUCCESS", "merge_request_event", json!([]))
        } } } })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({ "variables": { "sha": "3f2a91c4e", "ref": "main" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "project": { "pipelines": { "nodes": [
            pl("3f2a91c4e", "RUNNING", "push", json!([
                { "id": "gid://gitlab/Ci::Build/77", "name": "build", "status": "RUNNING", "startedAt": "2026-10-01T00:01:00Z",
                  "finishedAt": null, "webPath": "/platform/runner/-/jobs/77", "stage": { "name": "build" } },
                { "id": "gid://gitlab/Ci::Build/78", "name": "publish", "status": "MANUAL", "startedAt": null,
                  "finishedAt": null, "webPath": "/platform/runner/-/jobs/78", "stage": { "name": "deploy" } }
            ]))
        ] } } } })))
        .mount(&server)
        .await;
    let runs = gl(&server, &[]).runs(&id()).await.unwrap();
    let shape: Vec<(Which, &str, &str, CheckState)> =
        runs.iter().map(|r| (r.which, r.sha.as_str(), r.event.as_str(), r.state)).collect();
    assert_eq!(
        shape,
        [
            (Which::AfterMerge, "3f2a91c", "push", CheckState::Running),
            (Which::BeforeMerge, "9c01e7a", "merge request", CheckState::Passed)
        ]
    );
    let j = &runs[0].jobs;
    assert_eq!((j[0].id.as_str(), j[0].group.as_str(), j[0].state), ("77", "build", CheckState::Running));
    assert_eq!(j[1].state, CheckState::Queued, "a manual job has not run");
    assert!(j[0].url.ends_with("/platform/runner/-/jobs/77"));
}

#[tokio::test]
async fn a_jobs_log_is_asked_for_by_its_tail() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/platform%2Frunner/jobs/77/trace"))
        .and(wiremock::matchers::header("Range", format!("bytes=-{LOG_TAIL}").as_str()))
        .respond_with(ResponseTemplate::new(206).set_body_string("42\n"))
        .mount(&server)
        .await;
    let log = gl(&server, &[]).log(&id(), "77").await.unwrap();
    assert_eq!(log, "42\n", "a log that parses as JSON stays text");
}
