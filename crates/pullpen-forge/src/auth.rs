//! Where a token comes from (PLAN.md §1, ADR-006): the environment, then the
//! forge's own CLI, then a configured command. Nothing here writes a token
//! anywhere, and nothing logs one.

use std::process::Command;

use pullpen_core::Forge;

use crate::{Error, Result};

/// `env` is a parameter so tests can supply one; the binary passes
/// `std::env::var`.
pub fn token(kind: Forge, host: &str, token_cmd: Option<&str>, env: impl Fn(&str) -> Option<String>) -> Result<String> {
    let vars: &[&str] = match kind {
        Forge::GitHub => &["PULLPEN_GITHUB_TOKEN", "GITHUB_TOKEN"],
        Forge::GitLab => &["PULLPEN_GITLAB_TOKEN", "GITLAB_TOKEN"],
    };
    if let Some(t) = vars.iter().find_map(|v| env(v).filter(|t| !t.trim().is_empty())) {
        return Ok(t.trim().to_string());
    }
    let from_cli = match kind {
        Forge::GitHub => output("gh", &["auth", "token", "--hostname", host]),
        // glab prints the token on stderr, as "Token: …".
        Forge::GitLab => output("glab", &["auth", "status", "--hostname", host, "-t"]).and_then(|o| {
            o.lines()
                .find_map(|l| l.trim().strip_prefix("✓ Token: ").or(l.trim().strip_prefix("Token: ")).map(String::from))
        }),
    };
    if let Some(t) = from_cli.filter(|t| !t.is_empty() && !t.contains(' ')) {
        return Ok(t);
    }
    if let Some(cmd) = token_cmd
        && let Some(t) = output("sh", &["-c", cmd]).map(|o| o.trim().to_string()).filter(|t| !t.is_empty())
    {
        return Ok(t);
    }
    Err(Error::Auth(host.into()))
}

/// A command's output, stdout and stderr together, or `None` if it could not
/// run or failed. Failure here is normal (the CLI may not be installed), so it
/// is not an error.
fn output(bin: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(bin).args(args).output().ok()?;
    o.status.success().then(|| {
        let mut s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if s.is_empty() {
            s = String::from_utf8_lossy(&o.stderr).trim().to_string();
        }
        s
    })
}
