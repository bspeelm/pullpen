//! `pullpen setup`: the first-run walk through what pullpen needs to know
//! (PLAN.md §6.6). Every answer has a default, and running it again starts
//! from what was answered last time.

use std::io;

use pullpen_core::Forge;

use pullpen_forge::Place;

use crate::config::{Config, ForgeCfg, Kind, expand};
use crate::credentials::Credentials;
use crate::prompt::Io;

/// Who a token belongs to, and where they can watch PRs.
pub type SignedIn = (String, Vec<Place>);

/// What setup needs from outside, so a test can stand in for the network
/// and for `gh`.
pub struct Deps<'a> {
    /// Asks the forge who a token belongs to: the login and where they can
    /// watch PRs (their orgs, or their groups), or why not.
    pub verify: &'a mut dyn FnMut(Forge, &str, &str) -> Result<SignedIn, String>,
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
    (Forge::GitHub, "GitHub", "github.com", "gh", "Pull requests and Contents: read and write"),
    (Forge::GitLab, "GitLab", "gitlab.com", "glab", "api"),
];

/// Signs in to one forge. `Ok(None)` means the user chose to skip it.
fn sign_in(io: &mut Io, d: &mut Deps, f: Forge, host: &str, creds: &mut Credentials) -> io::Result<Option<Vec<Place>>> {
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
    if let Some(t) = creds.get(host).map(String::from)
        && io.yes(&format!("  Keep the token pullpen already has for {host}"), true)?
        && let Ok(places) = check(io, d, f, host, &t)?
    {
        return Ok(Some(places));
    }
    if let Some(t) = (d.cli)(f, host)
        && io.yes(&format!("  Use your {cli} login for {host}"), true)?
        && let Ok(places) = check(io, d, f, host, &t)?
    {
        creds.remove(host);
        return Ok(Some(places));
    }
    loop {
        match f {
            // Organisations can refuse pasted tokens and still accept the
            // CLI's OAuth login, so the login is the better first answer.
            Forge::GitHub => {
                io.say(&format!("  Easiest: Ctrl-C, run `{cli} auth login`, then `pullpen setup` again."))?;
                io.say(&format!("  Or make a fine-grained token for the repos you watch, with {scopes}:"))?;
                io.say(&format!("  https://{host}/settings/personal-access-tokens/new"))?;
            }
            Forge::GitLab => {
                io.say(&format!("  Make a token with the {scopes} scope, then paste it:"))?;
                io.say(&format!(
                    "  https://{host}/-/user_settings/personal_access_tokens?name=pullpen&scopes={scopes}"
                ))?;
            }
        }
        let t = io.hidden(&format!("  {name} token (blank to skip {host})"))?;
        if t.is_empty() {
            return Ok(None);
        }
        if t.starts_with("ghp_")
            && !io.yes(
                "  That is a classic token, which some organisations refuse: their PRs will not show. Use it anyway",
                false,
            )?
        {
            continue;
        }
        if let Ok(places) = check(io, d, f, host, &t)? {
            creds.set(host, t);
            return Ok(Some(places));
        }
        if !io.yes("  Try another token", true)? {
            return Ok(None);
        }
    }
}

fn check(io: &mut Io, d: &mut Deps, f: Forge, host: &str, token: &str) -> io::Result<Result<Vec<Place>, String>> {
    match (d.verify)(f, host, token) {
        Ok((login, places)) => {
            io.say(&format!("  ✓ signed in to {host} as {login}"))?;
            Ok(Ok(places))
        }
        Err(why) => {
            io.say(&format!("  ✗ {host}: {why}"))?;
            Ok(Err(why))
        }
    }
}

/// `a5434 (AIP)` where a name differs from its path, else the path.
fn shown(places: &[Place]) -> String {
    let one = |p: &Place| {
        let last = p.path.rsplit('/').next().unwrap_or(&p.path);
        if p.name.is_empty() || p.name.eq_ignore_ascii_case(last) {
            p.path.clone()
        } else {
            format!("{} ({})", p.path, p.name)
        }
    };
    places.iter().map(one).collect::<Vec<_>>().join("  ")
}

pub fn run(io: &mut Io, d: &mut Deps, mut cfg: Config, mut creds: Credentials) -> io::Result<(Config, Credentials)> {
    io.say("pullpen setup. Enter takes the value in [brackets]; run it again any time.\n")?;
    // One question per forge, defaulting to what is configured: adding
    // GitLab must not cost you GitHub, which a pick-one question did.
    let first_run = cfg.forge.is_empty();
    let had = |k: Kind| cfg.forge.iter().any(|f| f.kind == k);
    let mut forges = Vec::new();
    for (f, name, default_host, _, _) in NAMES {
        let default = if first_run { f == Forge::GitHub } else { had(kind_of(f)) };
        if !io.yes(&format!("Use {name}"), default)? {
            continue;
        }
        let old = cfg.forge.iter().find(|x| x.kind == kind_of(f)).cloned();
        io.say(&format!("\n{name}"))?;
        let host = io.line("  host", old.as_ref().map_or(default_host, |o| o.host()))?;
        let Some(places) = sign_in(io, d, f, &host, &mut creds)? else {
            io.say(&format!("  skipped {host}; `pullpen setup` again adds it"))?;
            continue;
        };
        let mut fc = ForgeCfg {
            kind: kind_of(f),
            host: (host != default_host).then(|| host.clone()),
            token_cmd: old.as_ref().and_then(|o| o.token_cmd.clone()),
            scope: None,
            groups: vec![],
        };
        if f == Forge::GitHub {
            let was = old.and_then(|o| o.scope).unwrap_or_default().replace("org:", "").replace("repo:", "");
            io.say("  Which PRs should lists like Open include?")?;
            io.say("  Blank: only PRs that involve you (you opened, review, are assigned or mentioned).")?;
            io.say("  Or name orgs, users or owner/repo to include every PR there.")?;
            if !places.is_empty() {
                io.say(&format!("  You can watch: {}", shown(&places)))?;
            }
            let a = io.line("  Watch", &was)?;
            let terms: Vec<String> = a
                .split_whitespace()
                .map(|t| if t.contains('/') { format!("repo:{t}") } else { format!("org:{t}") })
                .collect();
            fc.scope = (!terms.is_empty()).then(|| terms.join(" "));
        } else {
            let was = old.map(|o| o.groups.join(" ")).unwrap_or_default();
            let a = {
                io.say("  Which merge requests should lists like Open include?")?;
                io.say("  Blank: only ones that involve you (you opened, review or are assigned).")?;
                io.say("  Or name groups to include every merge request in them.")?;
                match places.is_empty() {
                    true => io.say(&format!("  You belong to no groups on {host}, so blank is the useful answer."))?,
                    false => io.say(&format!("  Your groups: {}", shown(&places)))?,
                }
                io.line("  Watch", &was)?
            };
            // A group you do not belong to shows nothing, which looks like a
            // bug later; say so now, while the answer can still change.
            fc.groups.clear();
            for g in a.split_whitespace() {
                // A group's name is what GitLab's pages show, and searches take
                // its path; either is accepted, and the path is what is kept.
                match places.iter().find(|p| p.path.eq_ignore_ascii_case(g) || p.name.eq_ignore_ascii_case(g)) {
                    Some(p) => {
                        if !p.path.eq_ignore_ascii_case(g) {
                            io.say(&format!("  {g} is the group at {}; saved as {}", p.path, p.path))?;
                        }
                        fc.groups.push(p.path.clone());
                    }
                    None if io.yes(
                        &format!("  {g} is not one of your groups and will show nothing; keep it anyway"),
                        false,
                    )? =>
                    {
                        fc.groups.push(g.to_string());
                    }
                    None => {}
                }
            }
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
    io.say("  When you press M or A, which merge method should pullpen use?")?;
    io.say("  (You can still pick another for any one merge: s, m or r at the confirm prompt.)")?;
    let labels = [
        "repo     whatever each repository is set to use (its default merge button)",
        "squash   always squash the PR into one commit",
        "merge    always make a merge commit",
        "rebase   always rebase the commits onto the base branch",
    ];
    cfg.merge.default_method = methods[io.pick("  Merge method", &labels, now)?].to_string();
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
            if !repo.contains('/') || repo.contains(' ') {
                io.say("  a repo is owner/name, like acme/api")?;
                continue;
            }
            let path = io.line("  path to its clone", cfg.repos.get(&repo).map_or("", String::as_str))?;
            if path.is_empty() {
                io.say("  no path given; skipped")?;
                continue;
            }
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
