use pullpen_core::query::Query;
use serde_json::json;
use wiremock::matchers::{body_partial_json, body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

fn gh(server: &MockServer) -> GitHub {
    GitHub::with_api(
        "github.com",
        &server.uri(),
        "t0ken".into(),
        Agents(Agents::default_logins()),
        "involves:@me",
        true,
    )
}

fn node() -> Value {
    json!({
        "number": 7, "title": "Bump deps", "url": "https://github.com/o/r/pull/7", "isDraft": false,
        "createdAt": "2026-10-01T00:00:00Z", "updatedAt": "2026-10-02T00:00:00Z",
        "additions": 10, "deletions": 2, "changedFiles": 3, "headRefName": "deps", "baseRefName": "main",
        "state": "OPEN", "mergedAt": null,
        "author": { "login": "renovate", "__typename": "Bot" },
        "repository": { "nameWithOwner": "o/r", "squashMergeAllowed": true, "mergeCommitAllowed": true,
                        "rebaseMergeAllowed": true, "viewerDefaultMergeMethod": "SQUASH" },
        "id": "PR_n7", "reviewDecision": "REVIEW_REQUIRED",
        "autoMergeRequest": { "mergeMethod": "SQUASH", "enabledAt": "2026-10-02T00:00:00Z", "enabledBy": { "login": "ash" } },
        "reviewRequests": { "nodes": [ { "requestedReviewer": { "__typename": "Team", "slug": "infra", "organization": { "login": "o" } } } ] },
        "latestOpinionatedReviews": { "nodes": [ { "state": "APPROVED", "author": { "login": "jkoh" } } ] },
        "commits": { "nodes": [ { "commit": { "statusCheckRollup": { "state": "PENDING", "contexts": { "totalCount": 3 } } } } ] }
    })
}

/// The follow-up the search asks for mergeability and check details.
fn details() -> Value {
    json!({ "data": { "nodes": [ {
        "id": "PR_n7", "mergeable": "MERGEABLE", "mergeStateStatus": "BEHIND",
        "commits": { "nodes": [ { "commit": { "statusCheckRollup": { "state": "PENDING", "contexts": { "nodes": [
            { "__typename": "CheckRun", "name": "ci", "status": "COMPLETED", "conclusion": "SUCCESS" },
            { "__typename": "CheckRun", "name": "e2e", "status": "IN_PROGRESS", "conclusion": null },
            { "__typename": "StatusContext", "context": "optional/lint", "state": "FAILURE" }
        ] } } } } ] }
    } ] } })
}

#[tokio::test]
async fn search_maps_a_pr_and_reads_requirements_from_rulesets() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("is:pr is:open review-requested:@me sort:updated-desc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "search": {
            "issueCount": 41, "pageInfo": { "hasNextPage": false, "endCursor": null }, "nodes": [node(), {}]
        } } })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_partial_json(json!({ "variables": { "ids": ["PR_n7"] } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(details()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/rules/branches/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "type": "pull_request", "parameters": { "required_approving_review_count": 2, "allowed_merge_methods": ["squash", "rebase"] } },
            { "type": "required_status_checks", "parameters": { "strict_required_status_checks_policy": true,
              "required_status_checks": [ { "context": "ci" }, { "context": "e2e" } ] } }
        ])))
        .expect(1)
        .mount(&server)
        .await;
    let g = gh(&server);
    let page = g.search(&Query::parse("is:open review-requested:@me"), "me", 50).await.unwrap();
    assert_eq!(page.total, 41);
    let p = &page.prs[0];
    assert_eq!((p.id.repo.as_str(), p.id.number), ("o/r", 7));
    assert!(p.author.is_agent);
    assert_eq!(
        (p.checks.required_total, p.checks.required_passed, p.checks.running),
        (2, 1, 1),
        "only required checks count"
    );
    assert!(p.checks.failed.is_empty(), "the failing check is not required");
    assert_eq!((p.reviews.have, p.reviews.need), (1, 2));
    assert_eq!(p.reviews.pending, vec!["o/infra".to_string()]);
    assert_eq!(p.mergeable, Mergeable::Behind);
    assert_eq!(p.repo_allowed_methods, vec![MergeMethod::Squash, MergeMethod::Rebase]);
    assert_eq!(p.auto_merge.as_ref().map(|a| a.armed_by.as_str()), Some("ash"));
    assert!(g.capabilities()["github.com/o/r"].requires_up_to_date);
    // The rules are cached per branch: a second search asks once more for
    // PRs only (the `.expect(1)` above holds on drop).
    g.search(&Query::parse("is:open review-requested:@me"), "me", 50).await.unwrap();
}

#[test]
fn an_unnarrowed_query_is_scoped_and_client_side_terms_stay_home() {
    let g = GitHub::with_api("github.com", "http://x", "t".into(), Agents::default(), "involves:@me", false);
    assert_eq!(g.server_query(&Query::parse("is:open")), "is:pr is:open involves:@me sort:updated-desc");
    assert_eq!(g.server_query(&Query::parse("is:armed ci:fail")), "is:pr involves:@me sort:updated-desc");
    assert_eq!(
        g.server_query(&Query::parse("is:open author:@me label:needs-review")),
        "is:pr is:open author:@me label:\"needs-review\" sort:updated-desc"
    );
}

#[tokio::test]
async fn merge_then_delete_the_branch_when_it_is_in_the_same_repo() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/repos/o/r/pulls/7/merge"))
        .and(body_partial_json(json!({ "merge_method": "squash" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "merged": true })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/pulls/7"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "head": { "ref": "deps", "repo": { "full_name": "o/r" } } })),
        )
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/repos/o/r/git/refs/heads/deps"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let id = PrId { forge: Forge::GitHub, host: "github.com".into(), repo: "o/r".into(), number: 7 };
    assert!(gh(&server).merge(&id, MergeMethod::Squash).await.unwrap(), "reports the branch deleted");
}

fn id() -> PrId {
    PrId { forge: Forge::GitHub, host: "github.com".into(), repo: "o/r".into(), number: 7 }
}

#[tokio::test]
async fn a_refused_merge_carries_the_forges_reason() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/repos/o/r/pulls/7/merge"))
        .respond_with(ResponseTemplate::new(405).set_body_json(json!({ "message": "Pull Request is not mergeable" })))
        .mount(&server)
        .await;
    let e = gh(&server).merge(&id(), MergeMethod::Squash).await.unwrap_err();
    assert_eq!(e, Error::Refused("Pull Request is not mergeable".into()));
}

#[tokio::test]
async fn arming_and_disarming_go_through_graphql_by_node_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("pullRequest(number"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "data": { "repository": { "pullRequest": { "id": "PR_n7" } } } })),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("enablePullRequestAutoMerge"))
        .and(body_partial_json(json!({ "variables": { "id": "PR_n7", "m": "REBASE" } })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "data": { "enablePullRequestAutoMerge": { "clientMutationId": null } } })),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("disablePullRequestAutoMerge"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "disablePullRequestAutoMerge": null },
            "errors": [ { "message": "Auto merge is not enabled for this pull request" } ]
        })))
        .mount(&server)
        .await;
    let g = gh(&server);
    g.arm(&id(), MergeMethod::Rebase).await.unwrap();
    let e = g.disarm(&id()).await.unwrap_err();
    assert_eq!(e, Error::Refused("Auto merge is not enabled for this pull request".into()));
}

#[tokio::test]
async fn approve_comment_and_close_hit_their_endpoints() {
    let server = MockServer::start().await;
    for (m, p, body) in [
        ("POST", "/repos/o/r/pulls/7/reviews", json!({ "event": "APPROVE" })),
        ("POST", "/repos/o/r/issues/7/comments", json!({ "body": "lgtm" })),
        ("PATCH", "/repos/o/r/pulls/7", json!({ "state": "closed" })),
    ] {
        Mock::given(method(m))
            .and(path(p))
            .and(body_partial_json(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;
    }
    let g = gh(&server);
    g.approve(&id()).await.unwrap();
    g.comment(&id(), "lgtm").await.unwrap();
    g.close(&id()).await.unwrap();
}

#[tokio::test]
async fn http_failures_map_to_what_the_ui_does_with_them() {
    let server = MockServer::start().await;
    let respond = |status: u16| ResponseTemplate::new(status).set_body_json(json!({ "message": "x" }));
    Mock::given(path("/repos/o/r/pulls/1/reviews")).respond_with(respond(401)).mount(&server).await;
    Mock::given(path("/repos/o/r/pulls/2/reviews")).respond_with(respond(502)).mount(&server).await;
    Mock::given(path("/repos/o/r/pulls/3/reviews"))
        .respond_with(respond(403).insert_header("x-ratelimit-remaining", "0").insert_header("retry-after", "8"))
        .mount(&server)
        .await;
    Mock::given(path("/repos/o/r/pulls/4/reviews"))
        .respond_with(respond(429).insert_header("retry-after", "3"))
        .mount(&server)
        .await;
    Mock::given(path("/repos/o/r/pulls/5/reviews"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({ "message": "Unprocessable Entity",
            "errors": ["Review Can not approve your own pull request"], "status": "422" })))
        .mount(&server)
        .await;
    let g = gh(&server);
    let at = |n| PrId { number: n, ..id() };
    assert_eq!(g.approve(&at(1)).await.unwrap_err(), Error::Auth("github.com".into()));
    assert_eq!(g.approve(&at(2)).await.unwrap_err(), Error::Server { host: "github.com".into(), status: 502 });
    assert_eq!(g.approve(&at(3)).await.unwrap_err(), Error::RateLimited { retry_in: 8 });
    assert_eq!(g.approve(&at(4)).await.unwrap_err(), Error::RateLimited { retry_in: 3 });
    assert_eq!(
        g.approve(&at(5)).await.unwrap_err(),
        Error::Refused("Review Can not approve your own pull request".into()),
        "GitHub's errors are plain strings here, and they, not \"Unprocessable Entity\", say why"
    );
}

#[test]
fn a_required_check_that_has_not_reported_is_pending() {
    let rollup = json!({ "state": "SUCCESS", "contexts": { "pageInfo": { "hasNextPage": false }, "nodes": [
        { "__typename": "CheckRun", "name": "lint", "status": "COMPLETED", "conclusion": "SUCCESS" }
    ] } });
    let c = checks(&rollup, &["required".to_string()]);
    assert_eq!((c.required_total, c.required_passed, c.running), (1, 0, 1));
    let cut = json!({ "state": "SUCCESS", "contexts": { "pageInfo": { "hasNextPage": true }, "nodes": [] } });
    let c = checks(&cut, &["required".to_string()]);
    assert_eq!((c.required_total, c.required_passed), (1, 1), "past a truncated list, the rollup decides");
}

#[tokio::test]
async fn detail_reads_checks_timeline_and_groups_pushes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("timelineItems"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "repository": { "pullRequest": {
            "body": "Line one.\r\nLine two.",
            "commits": { "nodes": [ { "commit": { "statusCheckRollup": { "contexts": { "nodes": [
                { "__typename": "CheckRun", "name": "build", "status": "COMPLETED", "conclusion": "SUCCESS",
                  "startedAt": "2026-10-01T00:00:00Z", "completedAt": "2026-10-01T00:01:52Z", "isRequired": true },
                { "__typename": "CheckRun", "name": "e2e", "status": "COMPLETED", "conclusion": "FAILURE",
                  "startedAt": "2026-10-01T00:00:10Z", "completedAt": "2026-10-01T00:08:14Z", "isRequired": false }
            ] } } } } ] },
            "timelineItems": { "totalCount": 9, "nodes": [
                { "__typename": "PullRequestCommit", "commit": { "messageHeadline": "a", "committedDate": "2026-10-01T00:00:00Z", "author": { "user": { "login": "claude" } } } },
                { "__typename": "PullRequestCommit", "commit": { "messageHeadline": "b", "committedDate": "2026-10-01T01:00:00Z", "author": { "user": { "login": "claude" } } } },
                { "__typename": "PullRequestReview", "state": "APPROVED", "body": "lgtm", "createdAt": "2026-10-01T02:00:00Z", "author": { "login": "jkoh" } }
            ] }
        } } } })))
        .mount(&server)
        .await;
    let d = gh(&server).detail(&id()).await.unwrap();
    assert_eq!(d.description, "Line one.\nLine two.");
    assert_eq!((d.checks[0].duration.as_str(), d.checks[0].required), ("1m 52s", true));
    assert_eq!(d.checks[1].state, CheckState::Failed);
    assert_eq!(d.checks_took, "8m 14s", "first start to last finish");
    assert_eq!(d.activity[0].kind, ActivityKind::Approved, "newest first");
    assert_eq!(d.activity[1].kind, ActivityKind::Pushed { commits: 2 }, "consecutive pushes by one author are one");
    assert_eq!(d.activity[1].who, "claude");
    assert_eq!(d.conversation_total, 9);
}

#[tokio::test]
async fn the_diff_is_asked_for_as_a_diff() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/pulls/7"))
        .and(wiremock::matchers::header("Accept", "application/vnd.github.diff"))
        .respond_with(ResponseTemplate::new(200).set_body_string("diff --git a/x b/x\n@@ -1 +1 @@\n-a\n+b\n"))
        .mount(&server)
        .await;
    let files = gh(&server).diff(&id()).await.unwrap();
    assert_eq!((files[0].path.as_str(), files[0].additions, files[0].deletions), ("x", 1, 1));
}

#[tokio::test]
async fn rows_the_token_may_not_read_are_counted_and_explained_not_dropped_silently() {
    let server = MockServer::start().await;
    let why = "`SimplySF` forbids access via a personal access token (classic). Please use a GitHub App, OAuth App, or a personal access token with fine-grained permissions.";
    Mock::given(method("POST"))
        .and(body_string_contains("search("))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "search": { "issueCount": 3, "pageInfo": { "hasNextPage": false }, "nodes": [null, null, null] } },
            "errors": [ { "message": why }, { "message": why }, { "message": why } ]
        })))
        .mount(&server)
        .await;
    let page = gh(&server).search(&Query::parse("is:open"), "me", 50).await.unwrap();
    assert_eq!((page.prs.len(), page.total), (0, 0), "the tab does not count what cannot be shown");
    assert_eq!(
        page.hidden.as_deref(),
        Some("SimplySF hides its PRs from classic tokens; run `pullpen setup` and use your gh login")
    );
}
