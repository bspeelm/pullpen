use crate::time::Timestamp;
use crate::*;

fn pr() -> Pr {
    Pr {
        id: PrId {
            forge: Forge::GitHub,
            host: "github.com".into(),
            repo: "acme/api".into(),
            number: 1,
        },
        title: "t".into(),
        author: Author {
            login: "a".into(),
            is_agent: false,
        },
        head: "h".into(),
        base: "main".into(),
        additions: 0,
        deletions: 0,
        files: 0,
        draft: false,
        checks: Checks {
            required_total: 2,
            required_passed: 2,
            running: 0,
            failed: vec![],
        },
        reviews: Reviews {
            have: 1,
            need: 1,
            changes_requested: false,
            pending: vec![],
        },
        mergeable: Mergeable::Clean,
        auto_merge: None,
        merge_train: None,
        state: PrState::Open,
        updated_at: Timestamp(0),
        created_at: Timestamp(0),
        url: String::new(),
        repo_default_method: MergeMethod::Squash,
        repo_allowed_methods: vec![MergeMethod::Squash],
    }
}

fn r(p: &Pr) -> Readiness {
    readiness(p, &Capabilities::default())
}

#[test]
fn green_clean_and_reviewed_is_ready() {
    assert_eq!(r(&pr()), Readiness::Ready);
}

#[test]
fn precedence_follows_the_design_order() {
    let mut p = pr();
    p.reviews.have = 0;
    assert_eq!(r(&p), Readiness::Waiting);
    p.checks.failed.push(CheckRun {
        name: "e2e".into(),
        detail: String::new(),
    });
    assert_eq!(r(&p), Readiness::Failing);
    p.mergeable = Mergeable::Conflict;
    assert_eq!(r(&p), Readiness::Conflict);
    p.draft = true;
    assert_eq!(r(&p), Readiness::Draft);
    p.state = PrState::Merged {
        at: Timestamp(0),
        method: MergeMethod::Squash,
    };
    assert_eq!(r(&p), Readiness::Merged);
}

#[test]
fn running_checks_wait() {
    let mut p = pr();
    p.checks.running = 1;
    p.checks.required_passed = 1;
    assert_eq!(r(&p), Readiness::Waiting);
}

#[test]
fn behind_base_blocks_only_where_the_repo_requires_up_to_date() {
    let mut p = pr();
    p.mergeable = Mergeable::Behind;
    assert_eq!(r(&p), Readiness::Ready);
    let strict = Capabilities {
        requires_up_to_date: true,
        ..Default::default()
    };
    assert_eq!(readiness(&p, &strict), Readiness::Waiting);
}

#[test]
fn changes_requested_blocks_even_with_enough_approvals() {
    let mut p = pr();
    p.reviews.changes_requested = true;
    assert_eq!(r(&p), Readiness::Failing);
}

#[test]
fn repo_name_is_the_last_segment_of_a_nested_gitlab_path() {
    let mut p = pr();
    p.id.repo = "group/sub/project".into();
    assert_eq!(p.id.repo_name(), "project");
}
