//! pullpen: one terminal queue for pull requests across GitHub and GitLab.

mod cli;
mod paths;
mod term;

use std::fs::{self, OpenOptions};
use std::sync::Mutex;

use anyhow::{Context, Result};
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let dirs = paths::Dirs::from_env()?;
    init_log(&dirs)?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        fixtures = args.fixtures,
        "start"
    );
    if !args.fixtures {
        // The forge layer arrives in M3 (PLAN.md §8); until then there is
        // nothing to show without fixtures, and saying so beats an empty list.
        anyhow::bail!("only --fixtures is available until the forge layer lands (PLAN.md §8, M3)");
    }
    term::run(&args)
}

/// Logs go to a file and never to stdout or stderr, which belong to the TUI.
fn init_log(dirs: &paths::Dirs) -> Result<()> {
    fs::create_dir_all(&dirs.state)
        .with_context(|| format!("creating {}", dirs.state.display()))?;
    let path = dirs.state.join("pullpen.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    tracing_subscriber::fmt()
        .with_writer(Mutex::new(file))
        .with_ansi(false)
        .init();
    Ok(())
}
