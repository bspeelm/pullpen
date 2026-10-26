use std::io::Cursor;

use super::*;

fn run_with(script: &str, cfg: Config, gh_login: bool, verify_ok: bool) -> (Config, Credentials, String) {
    let mut input = Cursor::new(script.as_bytes().to_vec());
    let mut out = Vec::new();
    let mut secret = || Ok("glpat_typed".to_string());
    let mut verify = |_: Forge, _: &str, t: &str| {
        if verify_ok { Ok(format!("me({})", &t[..4])) } else { Err("401 Unauthorized".to_string()) }
    };
    let cli = |f: Forge, _: &str| (gh_login && f == Forge::GitHub).then(|| "gho_cli".to_string());
    let env = |_: &str| None;
    let mut d = Deps { verify: &mut verify, cli: &cli, env: &env };
    let mut io = Io { input: &mut input, out: &mut out, secret: &mut secret };
    let (c, k) = run(&mut io, &mut d, cfg, Credentials::default()).unwrap();
    (c, k, String::from_utf8(out).unwrap())
}

#[test]
fn both_forges_with_the_gh_login_and_a_typed_gitlab_token() {
    // both; GitHub: host, use gh, scope; GitLab: host, (token is typed), groups;
    // merge: method, delete, bell, osc9; clones: no.
    let script = "3\n\ny\nacme platform/runner\n\nplatform\n2\n\nn\n\nn\n";
    let (c, k, out) = run_with(script, Config::default(), true, true);
    assert_eq!(c.forge.len(), 2);
    assert_eq!(c.forge[0].scope.as_deref(), Some("org:acme repo:platform/runner"));
    assert_eq!(c.forge[1].groups, vec!["platform".to_string()]);
    assert_eq!((c.merge.default_method.as_str(), c.merge.delete_branch, c.notify.bell), ("squash", true, false));
    assert_eq!(k.get("github.com"), None, "the gh login is used, not copied");
    assert_eq!(k.get("gitlab.com"), Some("glpat_typed"));
    assert!(out.contains("signed in to github.com as me(gho_)"));
    assert!(!out.contains("glpat_typed"), "a token is never printed");
}

#[test]
fn a_refused_token_can_be_skipped_and_nothing_is_saved() {
    // GitLab only; host; token refused; do not retry.
    let (c, k, out) = run_with("2\n\nn\n", Config::default(), false, false);
    assert!(out.contains("401 Unauthorized") && out.contains("nothing was saved"));
    assert!(c.forge.is_empty() && k.token.is_empty());
}

#[test]
fn clones_are_asked_for_and_checked() {
    // GitHub; host; gh login; no scope; merge defaults ×4; clones: yes, one
    // that is not a clone, kept; done; worktrees yes.
    let script = "1\n\ny\n\n\n\n\n\ny\ny\nacme/api\n/nowhere\ny\n\ny\n";
    let (c, _, out) = run_with(script, Config::default(), true, true);
    assert_eq!(c.repos.get("acme/api").map(String::as_str), Some("/nowhere"));
    assert!(c.checkout.worktree);
    assert!(out.contains("is not a git clone"));
    assert!(out.contains("a repo is owner/name"), "a bare y is not taken for a repo");
    assert!(!c.repos.contains_key("y"));
}

#[test]
fn a_classic_github_token_is_questioned_before_it_is_kept() {
    // GitHub; host; no gh login; the typed token is classic: decline it, then
    // skip with a blank one.
    let mut input = Cursor::new(b"1\n\nn\n".to_vec());
    let mut out = Vec::new();
    let mut answers = vec![String::new(), "ghp_classic".to_string()];
    let mut secret = || Ok(answers.pop().unwrap_or_default());
    let mut verify = |_: Forge, _: &str, _: &str| Ok("me".to_string());
    let cli = |_: Forge, _: &str| None;
    let env = |_: &str| None;
    let mut d = Deps { verify: &mut verify, cli: &cli, env: &env };
    let mut io = Io { input: &mut input, out: &mut out, secret: &mut secret };
    let (c, k) = run(&mut io, &mut d, Config::default(), Credentials::default()).unwrap();
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("classic token") && out.contains("settings/personal-access-tokens/new"));
    assert!(c.forge.is_empty() && k.token.is_empty(), "the declined classic token was not kept");
}
