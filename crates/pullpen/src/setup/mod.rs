//! `pullpen setup`: the first-run walk through what pullpen needs to know
//! (PLAN.md §6.6). Every answer has a default, and running it again starts
//! from what was answered last time.

use std::io;

use pullpen_core::Forge;

use crate::config::{Config, ForgeCfg, Kind, expand};
use crate::credentials::Credentials;
use crate::prompt::Io;

/// What setup needs from outside, so a test can stand in for the network
/// and for `gh`.
pub struct Deps<'a> {
    /// Asks the forge who a token belongs to: the login, or why not.
    pub verify: &'a mut dyn FnMut(Forge, &str, &str) -> Result<String, String>,
    /// A token from the forge's own CLI login, if there is one.
    pub cli: &'a dyn Fn(Forge, &str) -> Option<String>,
    pub env: &'a dyn Fn(&str) -> Option<String>,
}

fn kind_of(k: Forge) -> Kind {
    match k {
        Forge::GitHub => Kind::Github,
        Forge::GitLab => Kind::Gitlab,
    }
}

const NAMES: [(Forge, &str, &str, &str, &str); 2] = [
    (Forge::GitHub, "GitHub", "github.com", "gh", "repo, read:org"),
    (Forge::GitLab, "GitLab", "gitlab.com", "glab", "api"),
];

/// Signs in to one forge. `Ok(None)` means the user chose to skip it.
fn sign_in(io: &mut Io, d: &mut Deps, f: Forge, host: &str, creds: &mut Credentials) -> io::Result<Option<String>> {
    let (_, name, _, cli, scopes) = NAMES.iter().find(|n| n.0 == f).copied().unwrap_or(NAMES[0]);
    let var = if f == Forge::GitHub { "GITHUB_TOKEN" } else { "GITLAB_TOKEN" };
    let pvar = format!("PULLPEN_{var}");
    if let Some(t) = (d.env)(&pvar).or_else(|| (d.env)(var)) {
        io.say(&format!(
            "  using the token in ${} from your environment",
            if (d.env)(&pvar).is_some() { &pvar } else { var }
        ))?;
        return check(io, d, f, host, &t).map(|r| r.ok());
    }
    if let Some(t) = (d.cli)(f, host)
        && io.yes(&format!("  Use your {cli} login for {host}"), true)?
        && let Ok(login) = check(io, d, f, host, &t)?
    {
        creds.remove(host);
        return Ok(Some(login));
    }
    loop {
        let page = match f {
            Forge::GitHub => {
                format!("https://{host}/settings/tokens/new?scopes={}&description=pullpen", scopes.replace(", ", ","))
            }
            Forge::GitLab => {
                format!("https://{host}/-/user_settings/personal_access_tokens?name=pullpen&scopes={scopes}")
            }
        };
        io.say(&format!("  Make a token with the {scopes} scope(s) here, then paste it:\n  {page}"))?;
        let t = io.hidden(&format!("  {name} token (blank to skip {host})"))?;
        if t.is_empty() {
            return Ok(None);
        }
        if let Ok(login) = check(io, d, f, host, &t)? {
            creds.set(host, t);
            return Ok(Some(login));
        }
        if !io.yes("  Try another token", true)? {
            return Ok(None);
        }
    }
}

fn check(io: &mut Io, d: &mut Deps, f: Forge, host: &str, token: &str) -> io::Result<Result<String, String>> {
    let r = (d.verify)(f, host, token);
    match &r {
        Ok(login) => io.say(&format!("  ✓ signed in to {host} as {login}"))?,
        Err(why) => io.say(&format!("  ✗ {host}: {why}"))?,
    }
    Ok(r)
}

pub fn run(io: &mut Io, d: &mut Deps, mut cfg: Config, mut creds: Credentials) -> io::Result<(Config, Credentials)> {
    io.say("pullpen setup. Enter takes the value in [brackets]; run it again any time.\n")?;
    let had = |k: Kind| cfg.forge.iter().any(|f| f.kind == k);
    let start = match (had(Kind::Github), had(Kind::Gitlab)) {
        (_, true) if had(Kind::Github) => 2,
        (false, true) => 1,
        _ => 0,
    };
    let which = io.pick("Which forges do you use", &["GitHub", "GitLab", "both"], start)?;
    let mut forges = Vec::new();
    for (f, name, default_host, _, _) in NAMES {
        if which != 2 && which != (f as usize) {
            continue;
        }
        let old = cfg.forge.iter().find(|x| x.kind == kind_of(f)).cloned();
        io.say(&format!("\n{name}"))?;
        let host = io.line("  host", old.as_ref().map_or(default_host, |o| o.host()))?;
        if sign_in(io, d, f, &host, &mut creds)?.is_none() {
            io.say(&format!("  skipped {host}; `pullpen setup` again adds it"))?;
            continue;
        }
        let mut fc = ForgeCfg {
            kind: kind_of(f),
            host: (host != default_host).then_some(host),
            token_cmd: old.as_ref().and_then(|o| o.token_cmd.clone()),
            scope: None,
            groups: vec![],
        };
        if f == Forge::GitHub {
            let was = old.and_then(|o| o.scope).unwrap_or_default().replace("org:", "").replace("repo:", "");
            let a = io.line("  Orgs or repos to watch, space-separated (blank: PRs that involve you)", &was)?;
            let terms: Vec<String> = a
                .split_whitespace()
                .map(|t| if t.contains('/') { format!("repo:{t}") } else { format!("org:{t}") })
                .collect();
            fc.scope = (!terms.is_empty()).then(|| terms.join(" "));
        } else {
            let was = old.map(|o| o.groups.join(" ")).unwrap_or_default();
            let a =
                io.line("  Groups to watch, space-separated (blank: PRs you authored, review or are assigned)", &was)?;
            fc.groups = a.split_whitespace().map(String::from).collect();
        }
        forges.push(fc);
    }
    if forges.is_empty() {
        io.say("\nNo forge signed in, so nothing was saved.")?;
        return Ok((cfg, creds));
    }
    cfg.forge = forges;

    io.say("\nMerging")?;
    let methods = ["repo", "squash", "merge", "rebase"];
    let now = methods.iter().position(|m| *m == cfg.merge.default_method).unwrap_or(0);
    io.say("  How to merge when you do not say: the repo's own default, or always one method.")?;
    cfg.merge.default_method = methods[io.pick("  Default method", &methods, now)?].to_string();
    cfg.merge.delete_branch = io.yes("  Delete the branch after merging", cfg.merge.delete_branch)?;
    cfg.notify.bell = io.yes("  Ring the bell when an armed PR merges or needs you", cfg.notify.bell)?;
    cfg.notify.osc9 = io.yes("  Send a desktop notification as well", cfg.notify.osc9)?;

    io.say("\nLocal clones, for :checkout")?;
    if io.yes("  Tell pullpen where your clones are", !cfg.repos.is_empty())? {
        loop {
            let repo = io.line("  repo as owner/name (blank when done)", "")?;
            if repo.is_empty() {
                break;
            }
            let path = io.line("  path to its clone", cfg.repos.get(&repo).map_or("", String::as_str))?;
            if !expand(&path).join(".git").exists()
                && !io.yes(&format!("  {path} is not a git clone; keep it anyway"), false)?
            {
                continue;
            }
            cfg.repos.insert(repo, path);
        }
        cfg.checkout.worktree = io.yes("  Check out into a new worktree beside the clone", cfg.checkout.worktree)?;
    }
    Ok((cfg, creds))
}

#[cfg(test)]
mod tests;
