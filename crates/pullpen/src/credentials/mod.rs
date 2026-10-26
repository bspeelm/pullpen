//! Tokens `pullpen setup` was given (ADR-012). One file, readable only by
//! its owner, kept apart from config.toml so that file can be shared or
//! committed without carrying a secret.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Credentials {
    #[serde(default)]
    pub token: Vec<Stored>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Stored {
    pub host: String,
    pub token: String,
}

impl Credentials {
    pub fn get(&self, host: &str) -> Option<&str> {
        self.token.iter().find(|t| t.host == host).map(|t| t.token.as_str())
    }

    pub fn set(&mut self, host: &str, token: String) {
        self.token.retain(|t| t.host != host);
        self.token.push(Stored { host: host.into(), token });
    }

    pub fn remove(&mut self, host: &str) {
        self.token.retain(|t| t.host != host);
    }
}

/// No file is no stored tokens. A file others can read is refused rather
/// than used: its token may already have been read, and saying so is the
/// only useful thing left to do with it.
pub fn load(path: &Path) -> Result<Credentials, String> {
    let text = match fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Credentials::default()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
        Ok(t) => t,
    };
    // On Windows the file lives in %APPDATA%, whose ACL already admits only
    // its owner, and there is no mode to check without a new crate (ADR-016).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path).map_err(|e| e.to_string())?.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(format!(
                "{} can be read by others (mode {:o}); run `chmod 600 {}`",
                path.display(),
                mode & 0o777,
                path.display()
            ));
        }
    }
    toml::from_str(&text).map_err(|e| format!("{}: {}", path.display(), e.message()))
}

/// Written to a new file created owner-only, then renamed over the old one,
/// so the token is never on disk with wider permissions, even briefly.
pub fn save(path: &Path, c: &Credentials) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("toml.tmp");
    let _ = fs::remove_file(&tmp);
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
    let mut f = opts.open(&tmp)?;
    let body = toml::to_string(c).map_err(std::io::Error::other)?;
    f.write_all(format!("# Tokens pullpen setup stored. Owner-only; never share this file.\n\n{body}").as_bytes())?;
    f.sync_all()?;
    fs::rename(tmp, path)
}

#[cfg(test)]
mod tests;
