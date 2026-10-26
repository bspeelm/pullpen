use super::*;

#[test]
fn agents_are_listed_logins_bots_and_bot_suffixes() {
    let a = Agents(Agents::default_logins());
    assert!(a.is_agent("claude", false));
    assert!(a.is_agent("Dependabot", false));
    assert!(a.is_agent("renovate[bot]", false));
    assert!(a.is_agent("some-app[bot]", false));
    assert!(a.is_agent("ci-helper", true), "the forge's own bot flag counts");
    assert!(!a.is_agent("mara", false));
}

#[test]
fn only_retryable_failures_are_transient() {
    assert!(Error::RateLimited { retry_in: 5 }.transient());
    assert!(Error::Server { host: "h".into(), status: 502 }.transient());
    assert!(Error::Network("reset".into()).transient());
    assert!(!Error::Refused("no".into()).transient());
    assert!(!Error::Auth("h".into()).transient());
}

#[test]
fn tokens_come_from_the_environment_first() {
    let env = |k: &str| (k == "GITHUB_TOKEN").then(|| " ghp_x \n".to_string());
    assert_eq!(auth::token(pullpen_core::Forge::GitHub, "github.com", None, None, env).unwrap(), "ghp_x");
    let env = |k: &str| match k {
        "PULLPEN_GITLAB_TOKEN" => Some("first".to_string()),
        "GITLAB_TOKEN" => Some("second".to_string()),
        _ => None,
    };
    assert_eq!(auth::token(pullpen_core::Forge::GitLab, "gitlab.com", None, None, env).unwrap(), "first");
}

#[test]
fn a_stored_token_comes_after_the_environment_and_before_the_cli() {
    let none = |_: &str| None;
    let t = auth::token(pullpen_core::Forge::GitLab, "nowhere.invalid", Some("stored"), Some("echo from-cmd"), none)
        .unwrap();
    assert_eq!(t, "stored");
    let env = |k: &str| (k == "GITLAB_TOKEN").then(|| "env".to_string());
    assert_eq!(auth::token(pullpen_core::Forge::GitLab, "nowhere.invalid", Some("stored"), None, env).unwrap(), "env");
}

#[test]
fn a_configured_command_is_the_last_resort() {
    let none = |_: &str| None;
    let t = auth::token(pullpen_core::Forge::GitLab, "nowhere.invalid", None, Some("echo from-cmd"), none).unwrap();
    assert_eq!(t, "from-cmd");
}
