use super::*;
use pullpen_core::{Capabilities, verdict};

// The goldens cover the armed-and-waiting wording; these cover the branches
// no design screen reaches.

fn pr(n: u64) -> Pr {
    use pullpen_core::time::Timestamp;
    use pullpen_core::*;
    {
        Pr {
            id: PrId { forge: Forge::GitHub, host: "h".into(), repo: "o/r".into(), number: n },
            title: "t".into(),
            author: Author { login: "a".into(), is_agent: false },
            head: "x".into(),
            base: "main".into(),
            additions: 0,
            deletions: 0,
            files: 0,
            draft: false,
            checks: Checks { required_total: 9, required_passed: 4, running: 5, failed: vec![], ..Default::default() },
            reviews: Reviews { have: 0, need: 1, approved_by: vec![], changes_requested: false, pending: vec![] },
            mergeable: Mergeable::Clean,
            auto_merge: None,
            merge_train: None,
            state: PrState::Open,
            updated_at: Timestamp(0),
            created_at: Timestamp(0),
            url: String::new(),
            repo_default_method: MergeMethod::Squash,
            repo_allowed_methods: vec![MergeMethod::Squash],
            post_merge: None,
        }
    }
}

#[test]
fn waiting_and_unarmed_names_everything_and_offers_a() {
    let p = pr(1);
    let v = verdict(&p, &Capabilities::default());
    assert_eq!(verdict_long(&p, &v), "Waiting on CI (4/9) and 1 review. A arms auto-merge.");
    assert_eq!(verdict_short(&p, &v), "waiting on CI (4/9) and 1 review");
}

#[test]
fn ready_names_the_method() {
    let mut p = pr(1);
    p.checks.required_passed = 9;
    p.checks.running = 0;
    p.reviews.have = 1;
    let v = verdict(&p, &Capabilities::default());
    assert_eq!(verdict_long(&p, &v), "Nothing blocking. M merges with squash.");
}

#[test]
fn blocked_names_the_first_blocker_and_the_key_that_helps() {
    let mut p = pr(1);
    p.mergeable = pullpen_core::Mergeable::Conflict;
    let v = verdict(&p, &Capabilities::default());
    assert_eq!(verdict_long(&p, &v), "Conflicts with main. :checkout to resolve.");
}
