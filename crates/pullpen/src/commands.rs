//! `pullpen setup` and `pullpen settings` on a real terminal: the flows in
//! `setup` and `settings`, given stdin, the network and $EDITOR.

use std::io::{self, IsTerminal};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use pullpen_core::Forge;
use pullpen_forge::{Agents, auth};

use crate::cli::SettingsAction;
use crate::config::Config;
use crate::credentials::{self, Credentials};
use crate::prompt::{Io, read_hidden};
use crate::{config, settings, setup};

/// Whether a person is there to answer questions.
pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Asks the forge whose token this is: its answer is the only proof a
/// token works.
fn verify(f: Forge, host: &str, token: &str) -> Result<setup::SignedIn, String> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
    let t = token.to_string();
    rt.block_on(async move {
        let forge: Box<dyn pullpen_forge::Forge> = match f {
            Forge::GitHub => Box::new(pullpen_forge::github::GitHub::new(host, t, Agents::default(), "", false)),
            Forge::GitLab => Box::new(pullpen_forge::gitlab::GitLab::new(host, t, Agents::default(), vec![], false)),
        };
        let login = forge.viewer().await?;
        // Knowing where someone can watch is a help, not a requirement; a
        // token that cannot list groups still signs in.
        let places = forge.places().await.unwrap_or_default();
        Ok::<_, pullpen_forge::Error>((login, places))
    })
    .map_err(|e| e.to_string())
}

pub fn setup(cfg_path: &Path, creds_path: &Path, cfg: Config) -> Result<()> {
    let creds = credentials::load(creds_path).unwrap_or_default();
    let stdin = io::stdin();
    let (mut input, mut out) = (stdin.lock(), io::stdout());
    let mut secret = read_hidden;
    let mut io = Io { input: &mut input, out: &mut out, secret: &mut secret };
    let mut check = verify;
    let cli = |f: Forge, host: &str| auth::token(f, host, None, None, |_| None).ok();
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let mut deps = setup::Deps { verify: &mut check, cli: &cli, env: &env };
    let (cfg, creds) = setup::run(&mut io, &mut deps, cfg, creds)?;
    if cfg.forge.is_empty() {
        return Ok(());
    }
    cfg.save(cfg_path).with_context(|| format!("writing {}", cfg_path.display()))?;
    if !creds.token.is_empty() || creds_path.exists() {
        credentials::save(creds_path, &creds).with_context(|| format!("writing {}", creds_path.display()))?;
    }
    io.say(&format!("\nSaved {}.", cfg_path.display()))?;
    if !creds.token.is_empty() {
        io.say(&format!("Tokens are in {}, readable only by you.", creds_path.display()))?;
    }
    io.say("`pullpen settings` changes any of this later. Run `pullpen` to start.")?;
    Ok(())
}

/// Returns the exit status.
pub fn settings(cfg_path: &Path, creds_path: &Path, mut cfg: Config, action: Option<SettingsAction>) -> Result<i32> {
    match action {
        Some(SettingsAction::Path) => {
            println!("{}\n{}", cfg_path.display(), creds_path.display());
            Ok(0)
        }
        Some(SettingsAction::Get { key }) => match settings::get(&cfg, &key) {
            Ok(v) => {
                println!("{v}");
                Ok(0)
            }
            Err(e) => {
                eprintln!("{e}");
                Ok(1)
            }
        },
        Some(SettingsAction::Set { key, value }) => match settings::set(&mut cfg, &key, &value.join(" ")) {
            Ok(()) => {
                cfg.save(cfg_path)?;
                Ok(0)
            }
            Err(e) => {
                eprintln!("{e}");
                Ok(1)
            }
        },
        None => menu(cfg_path, creds_path, cfg),
    }
}

fn signed_in(cfg: &Config, creds: &Credentials) -> String {
    let parts: Vec<String> = cfg
        .forge
        .iter()
        .map(|f| format!("{}{}", f.host(), if creds.get(f.host()).is_some() { " (stored token)" } else { "" }))
        .collect();
    if parts.is_empty() { "nothing yet".into() } else { parts.join(", ") }
}

fn menu(cfg_path: &Path, creds_path: &Path, mut cfg: Config) -> Result<i32> {
    if !interactive() {
        eprintln!("pullpen settings needs a terminal; `pullpen settings get|set` works without one");
        return Ok(1);
    }
    loop {
        let creds = credentials::load(creds_path).unwrap_or_default();
        let stdin = io::stdin();
        let (mut input, mut out) = (stdin.lock(), io::stdout());
        let mut secret = read_hidden;
        let mut io = Io { input: &mut input, out: &mut out, secret: &mut secret };
        let mut save = |c: &Config| c.save(cfg_path);
        let who = signed_in(&cfg, &creds);
        match settings::menu(&mut io, &mut cfg, &who, &mut save)? {
            settings::Leave::Done => return Ok(0),
            settings::Leave::Setup => {
                drop(input);
                setup(cfg_path, creds_path, cfg)?;
            }
            settings::Leave::Edit => {
                drop(input);
                if !cfg_path.exists() {
                    cfg.save(cfg_path)?;
                }
                let editor =
                    std::env::var("VISUAL").or_else(|_| std::env::var("EDITOR")).unwrap_or_else(|_| "vi".into());
                let mut parts = editor.split_whitespace();
                let status = Command::new(parts.next().unwrap_or("vi")).args(parts).arg(cfg_path).status();
                if let Err(e) = status {
                    eprintln!("starting {editor}: {e}");
                }
                let (fresh, err) = Config::load(cfg_path);
                let found =
                    config::check::check(cfg_path, &fresh, err.as_deref(), Ok(&creds), &|k| std::env::var(k).ok());
                config::check::report(&found);
            }
        }
        cfg = Config::load(cfg_path).0;
    }
}
