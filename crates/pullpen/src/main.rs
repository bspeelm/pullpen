//! pullpen: one terminal queue for pull requests across GitHub and GitLab.

mod backend;
mod cache;
mod checkout;
mod cli;
mod commands;
mod config;
mod credentials;
mod fixtures;
mod keys;
mod live;
mod paths;
mod prompt;
mod settings;
mod setup;
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
    let creds_path = dirs.config.join("credentials.toml");
    let (mut cfg, mut cfg_err) = Config::load(&cfg_path);
    match args.command {
        Some(cli::Command::Config { action: cli::ConfigAction::Check }) => {
            let creds = credentials::load(&creds_path);
            let found = config::check::check(
                &cfg_path,
                &cfg,
                cfg_err.as_deref(),
                creds.as_ref().map_err(String::as_str),
                &|k| env::var(k).ok(),
            );
            std::process::exit(if config::check::report(&found) { 0 } else { 1 });
        }
        Some(cli::Command::Setup) => return commands::setup(&cfg_path, &creds_path, cfg),
        Some(cli::Command::Settings { action }) => {
            std::process::exit(commands::settings(&cfg_path, &creds_path, cfg, action)?)
        }
        None => {}
    }
    // A first run, with a person there to answer: ask before showing an
    // empty list. Without a terminal, start anyway; the defaults may suffice.
    if !args.fixtures && !cfg_path.exists() && commands::interactive() {
        println!("No config yet, so pullpen setup runs first.\n");
        commands::setup(&cfg_path, &creds_path, Config::default())?;
        (cfg, cfg_err) = Config::load(&cfg_path);
    }
    let theme = theme(&args, &cfg);
    if args.fixtures {
        let mut app = fixtures::app(theme);
        apply(&mut app, &cfg, &args);
        let mut backend = fixtures::Fake::new();
        let remap = remap(&mut app, &cfg);
        return term::run(term::Loop {
            app: &mut app,
            backend: &mut backend,
            dirs: &dirs,
            on_focus: false,
            bell: false,
            osc9: false,
            remap,
        });
    }
    let sections = cfg.sections();
    let visit = std::fs::read_to_string(dirs.state.join("last-visit")).ok().and_then(|t| t.trim().parse().ok());
    // A first run treats the last day as "since last visit".
    let since = pullpen_core::time::Timestamp(visit.unwrap_or(pullpen_core::time::Timestamp::now().0 - 86_400));
    let (creds, creds_err) = match credentials::load(&creds_path) {
        Ok(c) => (c, None),
        Err(e) => (credentials::Credentials::default(), Some(e)),
    };
    let (mut backend, forges, mut down) = live::Live::start(&cfg, &sections, dirs.cache.clone(), since, &creds)?;
    let states = sections
        .into_iter()
        .map(|s| {
            let (prs, total) = cache::load(&dirs.cache, &s).unwrap_or_default();
            SectionState::new(s, prs, total)
        })
        .collect();
    let mut app = App::new(states, theme, pullpen_core::time::Timestamp::now());
    if forges.is_empty() && down.is_empty() {
        down.push((pullpen_core::Forge::GitHub, "not signed in to any forge · run `pullpen setup`".into()));
    }
    // A forge that cannot be reached stays on the status line, marked ✗,
    // until it can: a message that one key clears is too easy to miss.
    app.forges = forges.into_iter().chain(down.iter().map(|(k, _)| *k)).collect();
    apply(&mut app, &cfg, &args);
    for (k, msg) in down {
        app.forge_down(k, msg);
    }
    for p in cfg_err.iter().chain(&creds_err) {
        app.error(p, Mode::Normal);
    }
    app.tz_offset = utc_offset();
    let remap = remap(&mut app, &cfg);
    term::run(term::Loop {
        app: &mut app,
        backend: &mut backend,
        dirs: &dirs,
        on_focus: cfg.sync.on_focus,
        bell: cfg.notify.bell,
        osc9: cfg.notify.osc9,
        remap,
    })
}

/// `[keys]`, with anything unreadable said on the command line.
fn remap(app: &mut App, cfg: &Config) -> Vec<(pullpen_ui::keymap::Key, pullpen_ui::keymap::Key)> {
    let (ok, bad) = pullpen_ui::keymap::remaps(&cfg.keys);
    for b in bad {
        app.error(&b, Mode::Normal);
    }
    ok
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
    // NO_COLOR (no-color.org): any non-empty value, unless a flag asks for
    // colours outright.
    let no_color = env::var("NO_COLOR").is_ok_and(|v| !v.is_empty());
    let tier = match (args.colors, cfg.ui.colors.as_str()) {
        (None, _) if no_color => Tier::Mono,
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
    palette.accent = cfg.ui.accent.as_deref().and_then(pullpen_ui::palette::parse_hex);
    // An unreadable palette file is reported by `pullpen config check`; the
    // built-in palette is the fallback.
    palette.custom = cfg
        .ui
        .palette
        .as_deref()
        .and_then(|p| std::fs::read_to_string(config::expand(p)).ok())
        .and_then(|t| pullpen_ui::palette::parse_file(&t).ok());
    Theme { palette, glyphs }
}

/// Seconds east of UTC, from `date +%z` (`+0200`). The standard library has
/// no time zones and none of the allowed crates does either (ADR-005); a
/// clock shown in UTC is the fallback, and only the armed view shows one.
fn utc_offset() -> i64 {
    let out = std::process::Command::new("date").arg("+%z").output().ok();
    let z = out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let (sign, digits) = match z.split_at_checked(1) {
        Some(("-", d)) => (-1, d),
        Some(("+", d)) => (1, d),
        _ => return 0,
    };
    let n: i64 = digits.parse().unwrap_or(0);
    sign * (n / 100 * 3600 + n % 100 * 60)
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
