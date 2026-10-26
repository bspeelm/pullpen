//! `:checkout`: the PR's head in a local clone (PLAN.md §6.3). The head is
//! fetched by the forge's PR ref, which works for forks as well as branches
//! in the repo itself.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use pullpen_core::{Forge, PrId};

/// Switches the clone of `id.repo` to `branch` at the PR's head, or adds a
/// worktree for it. Returns where, in words for the message.
///
/// An existing local branch is only ever fast-forwarded: if it has commits
/// the PR does not, it is left alone and the checkout refuses, because
/// resetting it would lose them.
pub fn checkout(
    id: &PrId,
    branch: &str,
    repos: &HashMap<String, String>,
    worktree: bool,
    cwd: &Path,
) -> Result<String, String> {
    let dir = clone_for(id, repos, cwd)?;
    let remote = remote_for(&dir, &id.repo).unwrap_or_else(|| "origin".into());
    let refspec = match id.forge {
        Forge::GitHub => format!("pull/{}/head", id.number),
        Forge::GitLab => format!("merge-requests/{}/head", id.number),
    };
    git(&dir, &["fetch", "--quiet", &remote, &refspec])?;
    let exists = git(&dir, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok();
    if worktree {
        let name = id.repo_name();
        let safe: String =
            branch.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }).collect();
        let path = dir.parent().unwrap_or(&dir).join(format!("{name}-{safe}"));
        let p = path.to_string_lossy().to_string();
        if exists {
            git(&dir, &["worktree", "add", &p, branch])?;
            git(&path, &["merge", "--ff-only", "--quiet", "FETCH_HEAD"]).map_err(|_| diverged(branch))?;
        } else {
            git(&dir, &["worktree", "add", "-b", branch, &p, "FETCH_HEAD"])?;
        }
        return Ok(format!("in worktree {}", tilde(&path)));
    }
    if exists {
        git(&dir, &["switch", "--quiet", branch])?;
        git(&dir, &["merge", "--ff-only", "--quiet", "FETCH_HEAD"]).map_err(|_| diverged(branch))?;
    } else {
        git(&dir, &["switch", "--quiet", "-c", branch, "FETCH_HEAD"])?;
    }
    Ok(format!("in {}", tilde(&dir)))
}

fn diverged(branch: &str) -> String {
    format!("local {branch} has commits the PR does not; left as it was")
}

/// The configured clone (`[repos]`), else the current directory if one of
/// its remotes points at the repo.
fn clone_for(id: &PrId, repos: &HashMap<String, String>, cwd: &Path) -> Result<PathBuf, String> {
    if let Some(p) = repos.get(&id.repo) {
        return Ok(expand(p));
    }
    if remote_for(cwd, &id.repo).is_some() {
        return Ok(cwd.to_path_buf());
    }
    Err(format!("no clone of {} known: add it under [repos] in config.toml, or run pullpen from inside it", id.repo))
}

/// The name of the remote whose URL ends in the repo path.
fn remote_for(dir: &Path, repo: &str) -> Option<String> {
    let out = git(dir, &["remote", "-v"]).ok()?;
    out.lines().find_map(|l| {
        let mut f = l.split_whitespace();
        let (name, url) = (f.next()?, f.next()?);
        let path = url.trim_end_matches('/').trim_end_matches(".git");
        (path.ends_with(&format!("/{repo}")) || path.ends_with(&format!(":{repo}"))).then(|| name.to_string())
    })
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git").arg("-C").arg(dir).args(args).output().map_err(|e| format!("running git: {e}"))?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&o.stderr);
        Err(err
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("git failed")
            .trim()
            .trim_start_matches("fatal: ")
            .to_string())
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn expand(p: &str) -> PathBuf {
    match (p.strip_prefix("~/"), home()) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => PathBuf::from(p),
    }
}

fn tilde(p: &Path) -> String {
    match home().and_then(|h| p.strip_prefix(h).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => p.display().to_string(),
    }
}

#[cfg(test)]
mod tests;
