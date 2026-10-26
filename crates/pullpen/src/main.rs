//! pullpen: one terminal queue for pull requests across GitHub and GitLab.

mod cli;
mod fixtures;
mod paths;
mod term;

use std::env;
use std::fs::{self, OpenOptions};
use std::sync::Mutex;

use anyhow::{Context, Result};
use clap::Parser;
use pullpen_ui::glyphs::Glyphs;
use pullpen_ui::paint::Theme;
use pullpen_ui::palette::{Palette, Tier, detect};

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
    let mut app = fixtures::app(theme(&args));
    term::run(&mut app, || pullpen_fixtures::NOW)
}

fn theme(args: &cli::Args) -> Theme {
    let tier = match args.colors {
        Some(cli::Colors::True) => Tier::True,
        Some(cli::Colors::C256) => Tier::C256,
        Some(cli::Colors::C16) => Tier::C16,
        None => detect(
            env::var("COLORTERM").ok().as_deref(),
            env::var("TERM").ok().as_deref(),
        ),
    };
    let glyphs = match args.glyphs {
        Some(cli::Glyphs::Ascii) => Glyphs::Ascii,
        Some(cli::Glyphs::Nerd) => Glyphs::Nerd,
        Some(cli::Glyphs::Unicode) | None => Glyphs::Unicode,
    };
    Theme {
        palette: Palette::new(tier),
        glyphs,
    }
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
