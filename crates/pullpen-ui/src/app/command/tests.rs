use super::*;

#[test]
fn parses_target_and_text() {
    let p = parse("comment #812 rebased on main, merging after the 2.4 cut");
    assert_eq!(p.name, "comment");
    assert_eq!(p.target, Some(TargetRef::Number { repo: None, number: 812 }));
    assert_eq!(p.rest, "rebased on main, merging after the 2.4 cut");
}

#[test]
fn gitlab_spelling_and_repo_prefix_are_targets() {
    assert_eq!(parse_target("!812"), Some(TargetRef::Number { repo: None, number: 812 }));
    assert_eq!(parse_target("runner#812"), Some(TargetRef::Number { repo: Some("runner".into()), number: 812 }));
    assert_eq!(parse_target("3"), Some(TargetRef::Row(3)));
    assert_eq!(parse_target("squash"), None);
}

#[test]
fn a_first_word_that_is_not_a_target_is_text() {
    let p = parse("comment lgtm");
    assert_eq!((p.target, p.rest), (None, "lgtm"));
}

#[test]
fn typos_suggest_the_nearest_command() {
    assert_eq!(did_you_mean("mrege"), Some("merge"));
    assert_eq!(did_you_mean("xyzzy"), None);
}

#[test]
fn aliases_parse() {
    assert_eq!(lookup("am"), Some(Cmd::Automerge));
    assert_eq!(lookup("nope"), None);
}
