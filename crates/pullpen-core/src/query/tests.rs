use super::*;

#[test]
fn parses_the_grammar_and_keeps_unknown_terms_as_text() {
    let q = Query::parse("is:open author:@me repo:acme/* ci:fail forge:gl bogus:x hydration");
    assert_eq!(
        q.terms,
        vec![
            Term::Is(Is::Open),
            Term::Author("@me".into()),
            Term::Repo("acme/*".into()),
            Term::Ci(Ci::Fail),
            Term::Forge(Forge::GitLab),
            Term::Text("bogus:x".into()),
            Term::Text("hydration".into()),
        ]
    );
    assert!(q.reaches(Forge::GitLab) && !q.reaches(Forge::GitHub));
}

#[test]
fn client_side_terms_are_the_ones_no_forge_search_has() {
    assert!(Term::Is(Is::Armed).client_side());
    assert!(Term::Author("@agents".into()).client_side());
    assert!(Term::Repo("acme/*".into()).client_side());
    assert!(!Term::Repo("acme/api".into()).client_side());
    assert!(!Term::Is(Is::Open).client_side());
}

#[test]
fn a_query_of_only_client_side_terms_is_not_narrowed() {
    assert!(!Query::parse("is:armed").narrowed());
    assert!(!Query::parse("is:open").narrowed());
    assert!(Query::parse("is:open review-requested:@me").narrowed());
}

#[test]
fn globs() {
    assert!(glob_match("acme/*", "acme/api"));
    assert!(glob_match("*/runner", "platform/runner"));
    assert!(glob_match("a*b*c", "aXbYc"));
    assert!(!glob_match("acme/*", "platform/api"));
    assert!(glob_match("exact", "exact"));
}

#[test]
fn merged_takes_a_window_and_keeps_anything_else_as_text() {
    assert_eq!(Query::parse("merged:7d").terms, vec![Term::MergedWithin(7 * 86_400)]);
    assert_eq!(Query::parse("merged:soon").terms, vec![Term::Text("merged:soon".into())]);
}
