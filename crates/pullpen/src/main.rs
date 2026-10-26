//! pullpen: one terminal queue for pull requests across GitHub and GitLab.

mod backend;
mod cache;
mod cli;
mod config;
mod fixtures;
mod keys;
mod live;
mod paths;
mod term;

use std::env;
use std::fs::{self, OpenOptions};
use std::sync::Mutex;

use anyhow::{Context, Result};
use clap::Parser;
use pullpen_ui::app::{App, DefaultMethod, PreviewMode, SectionState};
use pullpen_ui::glyphs::Glyphs;
use pullpen_ui::paint::Theme;
use pullpen_ui::palette::{Palette, Tier, detect};
use pullpen_ui::views::chrome::Mode;

use crate::config::Config;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let dirs = paths::Dirs::from_env()?;
    init_log(&dirs)?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), fixtures = args.fixtures, "start");
    let cfg_path = args.config.clone().unwrap_or_else(|| dirs.config.join("config.toml"));
    let (cfg, cfg_err) = Config::load(&cfg_path);
    let theme = theme(&args, &cfg);
    if args.fixtures {
        let mut app = fixtures::app(theme);
        apply(&mut app, &cfg, &args);
        let mut backend = fixtures::Fake::new();
        return term::run(term::Loop { app: &mut app, backend: &mut backend, dirs: &dirs, on_focus: false });
    }
    let sections = cfg.sections();
    let (mut backend, forges, problems) = live::Live::start(&cfg, &sections, dirs.cache.clone())?;
    let states = sections
        .into_iter()
        .map(|s| {
            let (prs, total) = cache::load(&dirs.cache, &s).unwrap_or_default();
            SectionState::new(s, prs, total)
        })
        .collect();
    let mut app = App::new(states, theme, pullpen_core::time::Timestamp::now());
    app.forges = forges;
    apply(&mut app, &cfg, &args);
    for p in problems.iter().chain(&cfg_err) {
        app.error(p, Mode::Normal);
    }
    if app.forges.is_empty() {
        app.error(
            "no forge to talk to: sign in with `gh auth login` or `glab auth login`, or set a token",
            Mode::Normal,
        );
    }
    term::run(term::Loop { app: &mut app, backend: &mut backend, dirs: &dirs, on_focus: cfg.sync.on_focus })
}

/// The config's choices that live in the app, and the starting section.
fn apply(app: &mut App, cfg: &Config, args: &cli::Args) {
    app.ui.default_method = cfg.merge.method().map_or(DefaultMethod::Repo, DefaultMethod::Fixed);
    app.ui.delete_branch = cfg.merge.delete_branch;
    app.ui.short_names_below = cfg.ui.short_names_below;
    app.ui.preview = match cfg.ui.preview.as_str() {
        "on" => PreviewMode::On,
        "off" => PreviewMode::Off,
        _ => PreviewMode::Auto,
    };
    if let Some(want) = &args.section {
        let w = want.to_lowercase();
        if let Some(i) =
            app.sections.iter().position(|s| s.section.name.to_lowercase() == w || s.section.short.to_lowercase() == w)
        {
            app.active = i;
        }
    }
}

fn theme(args: &cli::Args, cfg: &Config) -> Theme {
    let tier = match (args.colors, cfg.ui.colors.as_str()) {
        (Some(cli::Colors::True), _) | (None, "true") => Tier::True,
        (Some(cli::Colors::C256), _) | (None, "256") => Tier::C256,
        (Some(cli::Colors::C16), _) | (None, "16") => Tier::C16,
        _ => detect(env::var("COLORTERM").ok().as_deref(), env::var("TERM").ok().as_deref()),
    };
    let glyphs = match (args.glyphs, cfg.ui.glyphs.as_str()) {
        (Some(cli::Glyphs::Ascii), _) | (None, "ascii") => Glyphs::Ascii,
        (Some(cli::Glyphs::Nerd), _) | (None, "nerd") => Glyphs::Nerd,
        _ => Glyphs::Unicode,
    };
    let mut palette = Palette::new(tier);
    palette.accent = cfg.ui.accent.as_deref().and_then(rgb);
    Theme { palette, glyphs }
}

fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.strip_prefix('#')?;
    let n = u32::from_str_radix(h, 16).ok().filter(|_| h.len() == 6)?;
    Some(((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

/// Logs go to a file and never to stdout or stderr, which belong to the TUI.
fn init_log(dirs: &paths::Dirs) -> Result<()> {
    fs::create_dir_all(&dirs.state).with_context(|| format!("creating {}", dirs.state.display()))?;
    let path = dirs.state.join("pullpen.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    tracing_subscriber::fmt().with_writer(Mutex::new(file)).with_ansi(false).init();
    Ok(())
}
