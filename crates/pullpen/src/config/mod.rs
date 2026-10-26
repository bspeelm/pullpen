//! `config.toml` (PLAN.md §9). Everything has a default, so no file is a
//! valid config; a file that cannot be read is reported and the defaults
//! are used, rather than refusing to start.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use pullpen_core::{MergeMethod, Section, Sort, View};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
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
    pub repos: HashMap<String, String>,
    pub diff: Diff,
    pub keys: HashMap<String, String>,
    pub section: Vec<SectionCfg>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ForgeCfg {
    pub kind: Kind,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub token_cmd: Option<String>,
    /// GitHub: what an unscoped query is narrowed to.
    #[serde(default)]
    pub scope: Option<String>,
    /// GitLab: the groups an unscoped query searches.
    #[serde(default)]
    pub groups: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    pub glyphs: String,
    pub colors: String,
    pub accent: Option<String>,
    pub preview: String,
    pub short_names_below: usize,
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

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sync {
    pub interval: String,
    pub on_focus: bool,
}

impl Default for Sync {
    fn default() -> Self {
        Sync { interval: "60s".into(), on_focus: true }
    }
}

impl Sync {
    /// `60s`, `5m`, or a bare number of seconds; never under 10 s, which is
    /// as often as a forge's rate limit comfortably allows per section.
    pub fn every(&self) -> Duration {
        let s = self.interval.trim();
        let secs = match s.strip_suffix('m') {
            Some(m) => m.parse::<u64>().map(|m| m * 60).ok(),
            None => s.trim_end_matches('s').parse().ok(),
        };
        Duration::from_secs(secs.unwrap_or(60).max(10))
    }
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentsCfg {
    pub logins: Vec<String>,
}

impl Default for AgentsCfg {
    fn default() -> Self {
        AgentsCfg { logins: pullpen_forge::Agents::default_logins() }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Checkout {
    pub worktree: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Diff {
    pub pager: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SectionCfg {
    pub name: String,
    pub short: Option<String>,
    pub query: String,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub view: Option<String>,
    /// Whether the section feeds the status line's counts. Defaults to on,
    /// except for a bare `is:open`, whose failures are mostly not yours.
    #[serde(default)]
    pub counts: Option<bool>,
}

const DEFAULT_SECTIONS: [(&str, &str, &str, &str); 5] = [
    ("Mine", "Mine", "is:open author:@me", "list"),
    ("Needs my review", "Review", "is:open review-requested:@me", "list"),
    ("Agent-opened", "Agent", "is:open is:agent", "list"),
    ("Auto-merge armed", "Armed", "is:armed", "armed"),
    ("All", "All", "is:open", "list"),
];

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
                counts: c.counts.unwrap_or(c.query.trim() != "is:open"),
                sort: match c.sort.as_deref() {
                    Some("created") => Sort::Created,
                    Some("age") => Sort::Age,
                    Some("repo") => Sort::Repo,
                    _ => Sort::Updated,
                },
                view: if c.view.as_deref() == Some("armed") { View::Armed } else { View::List },
                name: c.name,
                query: c.query,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
