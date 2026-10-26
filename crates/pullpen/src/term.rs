//! Terminal lifecycle and the event loop.
//!
//! `ratatui::try_init` installs a panic hook that leaves raw mode and the
//! alternate screen before the panic message prints, so a crash never strands
//! the user in a broken terminal. `restore` runs on every normal exit path,
//! errors included.

use std::fs;
use std::io::{Write, stdout};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{
    self, DisableFocusChange, EnableFocusChange, Event, KeyEventKind, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate, supports_keyboard_enhancement};
use crossterm::{execute, queue};
use pullpen_core::{JobResult, PrId};
use pullpen_ui::app::{App, Effect};
use pullpen_ui::span::markup;
use pullpen_ui::views::chrome::Mode;
use ratatui::DefaultTerminal;

use crate::paths::Dirs;
use crate::{fixtures, keys};

/// How often the loop wakes without input: the spinner's frame rate, and
/// the resolution of the 500 ms key-sequence timeout.
const TICK: Duration = Duration::from_millis(100);
const HISTORY_MAX: usize = 500;

struct Term {
    kitty: bool,
}

impl Term {
    fn enter() -> Result<(DefaultTerminal, Term)> {
        let terminal = ratatui::try_init()?;
        // Where the terminal offers it, the Kitty keyboard protocol makes Tab
        // and ^i (and Esc and Alt-sequences) distinguishable (design §13).
        let kitty = supports_keyboard_enhancement().unwrap_or(false);
        if kitty {
            execute!(stdout(), PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES))?;
        }
        execute!(stdout(), EnableFocusChange)?;
        Ok((terminal, Term { kitty }))
    }

    /// Best effort: the terminal may already be gone, and an error here
    /// would only hide the one the caller is about to return.
    fn leave(&self) {
        let _ = execute!(stdout(), DisableFocusChange);
        if self.kitty {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
        ratatui::restore();
    }
}

pub fn run(app: &mut App, dirs: &Dirs) -> Result<()> {
    app.history = load_history(&dirs.state.join("history"));
    let (mut terminal, mut term) = Term::enter()?;
    let result = event_loop(&mut terminal, &mut term, app, dirs);
    term.leave();
    result
}

fn draw(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    // Synchronized output (mode 2026): the terminal shows the whole frame at
    // once instead of painting it as it arrives, which is what stops flicker.
    queue!(stdout(), BeginSynchronizedUpdate)?;
    terminal.draw(|f| {
        let area = f.area();
        app.render(area, f.buffer_mut());
    })?;
    queue!(stdout(), EndSynchronizedUpdate)?;
    stdout().flush()?;
    Ok(())
}

fn event_loop(terminal: &mut DefaultTerminal, term: &mut Term, app: &mut App, dirs: &Dirs) -> Result<()> {
    // Debug builds only: lets the panic path be exercised by hand
    // (`PULLPEN_TEST_PANIC=1 cargo run -- --fixtures`) without shipping a
    // crash switch in release binaries.
    #[cfg(debug_assertions)]
    if std::env::var_os("PULLPEN_TEST_PANIC").is_some() {
        draw(terminal, app)?;
        panic!("PULLPEN_TEST_PANIC is set");
    }
    let (tx, rx) = mpsc::channel::<JobResult>();
    while !app.quit {
        draw(terminal, app)?;
        let mut effects = Vec::new();
        if event::poll(TICK)? {
            match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => {
                    if let Some(key) = keys::convert(k) {
                        effects.extend(app.key(key, Instant::now()));
                    }
                }
                // A resize needs nothing beyond the redraw at the top of the
                // loop: ratatui re-reads the size on every draw.
                _ => {}
            }
        } else {
            app.expire(Instant::now());
            if !app.jobs.is_empty() {
                app.tick += 1;
            }
        }
        while let Ok(r) = rx.try_recv() {
            effects.extend(app.job_done(r));
        }
        for e in effects {
            run_effect(e, terminal, term, app, dirs, &tx)?;
        }
    }
    Ok(())
}

fn run_effect(
    e: Effect,
    terminal: &mut DefaultTerminal,
    term: &mut Term,
    app: &mut App,
    dirs: &Dirs,
    tx: &mpsc::Sender<JobResult>,
) -> Result<()> {
    match e {
        Effect::Run(job) => fixtures::run(job, tx.clone()),
        Effect::Edit { targets } => {
            term.leave();
            let text = edit(dirs, &targets);
            let (t, k) = Term::enter()?;
            *terminal = t;
            *term = k;
            terminal.clear()?;
            match text {
                Ok(text) => {
                    for e in app.comment_text(targets, text) {
                        if let Effect::Run(job) = e {
                            fixtures::run(job, tx.clone());
                        }
                    }
                }
                Err(err) => app.error(&format!("{err:#}"), Mode::Normal),
            }
        }
        Effect::Open(url) => app.say(markup(&format!("{{d|fixtures: would open {url}}}")), Mode::Normal),
        Effect::Refresh { .. } => app.sync = "synced 0s".into(),
        Effect::Redraw => terminal.clear()?,
        Effect::SaveHistory => save_history(&dirs.state.join("history"), &app.history)?,
    }
    Ok(())
}

/// Opens $EDITOR (or $VISUAL, or vi) on a scratch file in the cache
/// directory and returns what was written, minus the comment lines.
fn edit(dirs: &Dirs, targets: &[PrId]) -> Result<String> {
    fs::create_dir_all(&dirs.cache)?;
    let path = dirs.cache.join("COMMENT.md");
    let refs: Vec<String> = targets.iter().map(|t| format!("{}#{}", t.repo, t.number)).collect();
    fs::write(
        &path,
        format!(
            "\n# Comment on {}. Lines starting with # are dropped; an empty comment posts nothing.\n",
            refs.join(", ")
        ),
    )?;
    let editor = std::env::var("VISUAL").or_else(|_| std::env::var("EDITOR")).unwrap_or_else(|_| "vi".into());
    let mut parts = editor.split_whitespace();
    let bin = parts.next().unwrap_or("vi");
    let status = Command::new(bin).args(parts).arg(&path).status().with_context(|| format!("starting {editor}"))?;
    anyhow::ensure!(status.success(), "{editor} exited with {status}; nothing posted");
    let text = fs::read_to_string(&path)?;
    // Removing the scratch file is tidiness; a leftover is overwritten next time.
    let _ = fs::remove_file(&path);
    Ok(text.lines().filter(|l| !l.starts_with('#')).collect::<Vec<_>>().join("\n").trim().to_string())
}

fn load_history(path: &Path) -> Vec<String> {
    fs::read_to_string(path).map(|t| t.lines().map(String::from).collect()).unwrap_or_default()
}

fn save_history(path: &Path, h: &[String]) -> Result<()> {
    let keep = &h[h.len().saturating_sub(HISTORY_MAX)..];
    fs::write(path, keep.join("\n") + "\n").with_context(|| format!("writing {}", path.display()))
}
