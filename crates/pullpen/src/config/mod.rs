//! `config.toml` (PLAN.md §9). Everything has a default, so no file is a
//! valid config; a file that cannot be read is reported and the defaults
//! are used, rather than refusing to start.

pub mod check;

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use pullpen_core::{MergeMethod, Section, Sort, View};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
#[derive(Default)]
pub struct Config {
    pub forge: Vec<ForgeCfg>,
    pub ui: Ui,
    pub sync: Sync,
    pub merge: Merge,
    pub notify: Notify,
    pub agents: AgentsCfg,
    pub checkout: Checkout,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub repos: HashMap<String, String>,
    pub diff: Diff,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub keys: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub section: Vec<SectionCfg>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ForgeCfg {
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_cmd: Option<String>,
    /// GitHub: what an unscoped query is narrowed to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// GitLab: the groups an unscoped query searches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Github,
    Gitlab,
}

impl ForgeCfg {
    pub fn host(&self) -> &str {
        self.host.as_deref().unwrap_or(match self.kind {
            Kind::Github => "github.com",
            Kind::Gitlab => "gitlab.com",
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    pub glyphs: String,
    pub colors: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    pub preview: String,
    pub short_names_below: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
}

impl Default for Ui {
    fn default() -> Self {
        Ui {
            glyphs: "unicode".into(),
            colors: "auto".into(),
            accent: None,
            preview: "auto".into(),
            short_names_below: 100,
            palette: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sync {
    pub interval: String,
    pub on_focus: bool,
    /// How often an open pipeline screen asks again while a job runs.
    pub watch: String,
}

impl Default for Sync {
    fn default() -> Self {
        Sync { interval: "60s".into(), on_focus: true, watch: "5s".into() }
    }
}

/// `60s`, `5m`, or a bare number of seconds.
fn secs(s: &str) -> Option<u64> {
    let s = s.trim();
    match s.strip_suffix('m') {
        Some(m) => m.parse::<u64>().map(|m| m * 60).ok(),
        None => s.trim_end_matches('s').parse().ok(),
    }
}

impl Sync {
    /// Never under 10 s, which is as often as a forge's rate limit
    /// comfortably allows per section.
    pub fn every(&self) -> Duration {
        Duration::from_secs(secs(&self.interval).unwrap_or(60).max(10))
    }

    /// Never under 2 s: one pipeline, one log, while someone watches.
    pub fn watch_every(&self) -> Duration {
        Duration::from_secs(secs(&self.watch).unwrap_or(5).max(2))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Merge {
    pub default_method: String,
    pub delete_branch: bool,
}

impl Default for Merge {
    fn default() -> Self {
        Merge { default_method: "repo".into(), delete_branch: true }
    }
}

impl Merge {
    pub fn method(&self) -> Option<MergeMethod> {
        match self.default_method.as_str() {
            "squash" => Some(MergeMethod::Squash),
            "merge" => Some(MergeMethod::Merge),
            "rebase" => Some(MergeMethod::Rebase),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Notify {
    pub bell: bool,
    pub osc9: bool,
}

impl Default for Notify {
    fn default() -> Self {
        Notify { bell: true, osc9: true }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentsCfg {
    pub logins: Vec<String>,
}

impl Default for AgentsCfg {
    fn default() -> Self {
        AgentsCfg { logins: pullpen_forge::Agents::default_logins() }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Checkout {
    pub worktree: bool,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Diff {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pager: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SectionCfg {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// Whether the section feeds the status line's counts. Defaults to on,
    /// except for a bare `is:open`, whose failures are mostly not yours.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counts: Option<bool>,
}

const DEFAULT_SECTIONS: [(&str, &str, &str, &str); 6] = [
    ("Mine", "Mine", "is:open author:@me", "list"),
    ("Needs my review", "Review", "is:open review-requested:@me", "list"),
    ("Agent-opened", "Agent", "is:open is:agent", "list"),
    ("Auto-merge armed", "Armed", "is:armed", "armed"),
    ("All", "All", "is:open", "list"),
    ("Merged", "Merged", "is:merged merged:7d", "merged"),
];

/// The home directory: $HOME, or on Windows, where it is usually unset,
/// %USERPROFILE%.
pub fn home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(std::path::PathBuf::from)
}

/// `~/x` (or `~\x`) as a path under the home directory.
pub fn expand(p: &str) -> std::path::PathBuf {
    match (p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")), home()) {
        (Some(rest), Some(h)) => h.join(rest),
        _ => std::path::PathBuf::from(p),
    }
}

impl Config {
    /// The config, and the reason it could not be read if it could not.
    pub fn load(path: &Path) -> (Config, Option<String>) {
        match std::fs::read_to_string(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), None),
            Err(e) => (Config::default(), Some(format!("{}: {e}; using defaults", path.display()))),
            Ok(text) => match toml::from_str::<Config>(&text) {
                Ok(c) => (c, None),
                Err(e) => (Config::default(), Some(format!("{}: {}; using defaults", path.display(), e.message()))),
            },
        }
    }

    /// Writes the config whole, after keeping the previous file as
    /// `config.toml.bak`. Comments in a hand-written file do not survive a
    /// save: the toml writer cannot keep them, and the backup does.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        if path.exists() {
            std::fs::copy(path, path.with_extension("toml.bak"))?;
        }
        let body = toml::to_string(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(
            &tmp,
            format!(
                "# Written by pullpen settings and pullpen setup; edit freely.\n# Tokens are not here: they are in credentials.toml, beside it.\n\n{body}"
            ),
        )?;
        std::fs::rename(tmp, path)
    }

    /// Configured forges, or both public ones when none are configured.
    pub fn forges(&self) -> Vec<ForgeCfg> {
        if !self.forge.is_empty() {
            return self.forge.clone();
        }
        [Kind::Github, Kind::Gitlab]
            .map(|kind| ForgeCfg { kind, host: None, token_cmd: None, scope: None, groups: vec![] })
            .to_vec()
    }

    pub fn sections(&self) -> Vec<Section> {
        let cfgs: Vec<SectionCfg> = if self.section.is_empty() {
            DEFAULT_SECTIONS
                .iter()
                .map(|(n, s, q, v)| SectionCfg {
                    name: n.to_string(),
                    short: Some(s.to_string()),
                    query: q.to_string(),
                    sort: None,
                    view: Some(v.to_string()),
                    counts: None,
                })
                .collect()
        } else {
            self.section.clone()
        };
        cfgs.into_iter()
            .map(|c| Section {
                short: c.short.clone().unwrap_or_else(|| c.name.clone()),
                // A catch-all list counts nothing; nor do merged PRs, whose
                // failing pipelines are not PRs anyone can still fix by merging.
                counts: c.counts.unwrap_or(c.query.trim() != "is:open" && c.view.as_deref() != Some("merged")),
                sort: match c.sort.as_deref() {
                    Some("created") => Sort::Created,
                    Some("age") => Sort::Age,
                    Some("repo") => Sort::Repo,
                    _ => Sort::Updated,
                },
                view: match c.view.as_deref() {
                    Some("armed") => View::Armed,
                    Some("merged") => View::Merged,
                    _ => View::List,
                },
                name: c.name,
                query: c.query,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
