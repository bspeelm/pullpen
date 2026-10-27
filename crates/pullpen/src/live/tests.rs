use std::fs;
use std::path::Path;
use std::sync::Mutex;

use pullpen_core::query::Query;
use pullpen_core::{Section, Sort, View};
use pullpen_forge::github::GitHub;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const TOKEN: &str = "ghp_SECRET_never_on_disk_7f3a";

fn files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() { files(&p, out) } else { out.push(p) }
    }
}

/// ADR-006: a token pullpen was given appears in nothing it writes — not
/// the log, at any level, not the cache, not the history — including when a
/// request fails and its error is reported.
#[test]
fn a_token_never_reaches_a_file() {
    let dir = std::env::temp_dir().join(format!("pullpen-token-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let log = fs::File::create(dir.join("pullpen.log")).unwrap();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(Mutex::new(log))
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let rt = tokio::runtime::Runtime::new().unwrap();
    tracing::subscriber::with_default(subscriber, || {
        rt.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(body_string_contains("search("))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "search": {
                    "issueCount": 1, "pageInfo": { "hasNextPage": false },
                    "nodes": [ { "id": "PR_1", "number": 1, "title": "t", "state": "OPEN",
                                 "repository": { "nameWithOwner": "o/r" }, "baseRefName": "main", "author": { "login": "a" } } ]
                } } })))
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(body_string_contains("nodes(ids"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "nodes": [ { "id": "PR_1" } ] } })))
                .mount(&server)
                .await;
            Mock::given(path("/repos/o/r/pulls/1/reviews")).respond_with(ResponseTemplate::new(401)).mount(&server).await;
            let g = GitHub::with_api("github.com", &server.uri(), TOKEN.into(), Agents::default(), "involves:@me", false);
            let page = g.search(&Query::parse("is:open"), "me", 10).await.unwrap();
            let section = Section { name: "All".into(), short: "All".into(), query: "is:open".into(), sort: Sort::Updated, view: View::List, counts: true };
            cache::store(&dir, &section, &page.prs, page.total).unwrap();
            let id = page.prs[0].id.clone();
            let err = g.approve(&id).await.unwrap_err();
            tracing::error!(%err, "approve failed");
            assert!(!err.to_string().contains(TOKEN));
        });
    });
    let mut all = Vec::new();
    files(&dir, &mut all);
    assert!(all.len() >= 2, "expected a log and a cache file, found {all:?}");
    let log = fs::read_to_string(dir.join("pullpen.log")).unwrap();
    assert!(log.contains("approve failed"), "the log was written, so the check below means something");
    for f in &all {
        let text = String::from_utf8_lossy(&fs::read(f).unwrap()).to_string();
        assert!(!text.contains(TOKEN), "{} contains the token", f.display());
    }
    fs::remove_dir_all(dir).unwrap();
}

/// Ages count from the moment drawn, not from launch: the loop sets
/// `App::now` from this before every frame.
#[test]
fn the_clock_is_the_wall_clock() {
    let dir = std::env::temp_dir().join(format!("pullpen-clock-{}", std::process::id()));
    let (live, _, _) = Live::start(&Config::default(), &[], dir, Timestamp(0), &Credentials::default()).unwrap();
    assert!((live.now().0 - Timestamp::now().0).abs() <= 1);
}
