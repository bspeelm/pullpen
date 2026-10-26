//! Where pullpen reads and writes. XDG on every platform, macOS included,
//! so the documented paths are the real ones everywhere; these three
//! directories are the only places pullpen writes.

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
        Self::resolve(|k| env::var_os(k).map(PathBuf::from))
    }

    fn resolve(var: impl Fn(&str) -> Option<PathBuf>) -> Result<Self> {
        let Some(home) = var("HOME").filter(|p| p.is_absolute()) else {
            bail!("HOME is unset or not absolute; pullpen cannot place its files");
        };
        // The XDG spec says a relative value is invalid and must be ignored.
        let xdg = |k: &str, fallback: &str| {
            var(k)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(fallback))
                .join("pullpen")
        };
        Ok(Dirs {
            config: xdg("XDG_CONFIG_HOME", ".config"),
            cache: xdg("XDG_CACHE_HOME", ".cache"),
            state: xdg("XDG_STATE_HOME", ".local/state"),
        })
    }
}

#[cfg(test)]
mod tests;
