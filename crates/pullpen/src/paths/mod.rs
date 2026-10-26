//! Where pullpen reads and writes. XDG on Linux and macOS, so the documented
//! paths are the real ones there; Windows' own folders on Windows, where
//! nothing looks in ~/.config (ADR-016). These three directories are the
//! only places pullpen writes.

use std::env;
use std::path::PathBuf;

use anyhow::{Result, bail};

#[derive(Debug, PartialEq)]
pub struct Dirs {
    pub config: PathBuf,
    pub cache: PathBuf,
    pub state: PathBuf,
}

impl Dirs {
    pub fn from_env() -> Result<Self> {
        Self::resolve(|k| env::var_os(k).map(PathBuf::from), cfg!(windows))
    }

    fn resolve(var: impl Fn(&str) -> Option<PathBuf>, windows: bool) -> Result<Self> {
        // XDG variables still win on Windows, so a setup that sets them on
        // purpose (Git Bash, MSYS) is obeyed.
        let set = |k: &str| var(k).filter(|p| p.is_absolute());
        if windows {
            let (Some(roaming), Some(local)) = (
                var("APPDATA").filter(|p| !p.as_os_str().is_empty()),
                var("LOCALAPPDATA").filter(|p| !p.as_os_str().is_empty()),
            ) else {
                bail!("APPDATA or LOCALAPPDATA is unset; pullpen cannot place its files");
            };
            let pp = |k: &str, base: &PathBuf, sub: &str| {
                set(k).map_or_else(|| base.join("pullpen").join(sub), |p| p.join("pullpen"))
            };
            return Ok(Dirs {
                config: set("XDG_CONFIG_HOME").map_or_else(|| roaming.join("pullpen"), |p| p.join("pullpen")),
                cache: pp("XDG_CACHE_HOME", &local, "cache"),
                state: pp("XDG_STATE_HOME", &local, "state"),
            });
        }
        let Some(home) = var("HOME").filter(|p| p.is_absolute()) else {
            bail!("HOME is unset or not absolute; pullpen cannot place its files");
        };
        // The XDG spec says a relative value is invalid and must be ignored.
        let xdg = |k: &str, fallback: &str| set(k).unwrap_or_else(|| home.join(fallback)).join("pullpen");
        Ok(Dirs {
            config: xdg("XDG_CONFIG_HOME", ".config"),
            cache: xdg("XDG_CACHE_HOME", ".cache"),
            state: xdg("XDG_STATE_HOME", ".local/state"),
        })
    }
}

#[cfg(test)]
mod tests;
