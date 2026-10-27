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
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{
    self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event, KeyEventKind,
    KeyboardEnhancementFlags, MouseButton, MouseEventKind, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::style::Print;
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate, SetTitle, supports_keyboard_enhancement};
use crossterm::{execute, queue};
use pullpen_core::PrId;
use pullpen_ui::app::{App, Effect, Mouse};
use pullpen_ui::views::chrome::Mode;
use ratatui::DefaultTerminal;

use crate::backend::{Backend, Msg};
use crate::keys;
use crate::paths::Dirs;

/// How often the loop wakes without input: the spinner's frame rate, and
/// the resolution of the 500 ms key-sequence timeout.
const TICK: Duration = Duration::from_millis(100);
const HISTORY_MAX: usize = 500;

struct Term {
    kitty: bool,
    /// The window title last sent, so it is sent again only when it changes.
    title: String,
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
        execute!(stdout(), EnableFocusChange, EnableMouseCapture)?;
        // Save the window title on the terminal's own stack (XTWINOPS 22),
        // so leaving can put back whatever the shell had; a terminal without
        // the stack ignores it, and the shell's next prompt retitles anyway.
        execute!(stdout(), Print("\x1b[22;0t"), SetTitle("pullpen"))?;
        Ok((terminal, Term { kitty, title: "pullpen".into() }))
    }

    /// Best effort: the terminal may already be gone, and an error here
    /// would only hide the one the caller is about to return.
    fn leave(&self) {
        let _ = execute!(stdout(), DisableFocusChange, DisableMouseCapture, Print("\x1b[23;0t"));
        if self.kitty {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
        ratatui::restore();
    }
}

pub struct Loop<'a> {
    pub app: &'a mut App,
    pub backend: &'a mut dyn Backend,
    pub dirs: &'a Dirs,
    pub on_focus: bool,
    pub bell: bool,
    pub osc9: bool,
    /// `[keys]`: a pressed key and the key it stands for.
    pub remap: Vec<(pullpen_ui::keymap::Key, pullpen_ui::keymap::Key)>,
}

pub fn run(mut l: Loop) -> Result<()> {
    l.app.history = load_history(&l.dirs.state.join("history"));
    // An unreadable list only brings back rows that were cleared; :clear again.
    let cleared = fs::read_to_string(l.dirs.state.join("cleared.json")).ok();
    l.app.set_cleared(cleared.and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default());
    let visit = l.dirs.state.join("last-visit");
    l.app.last_visit =
        fs::read_to_string(&visit).ok().and_then(|t| t.trim().parse().ok()).map(pullpen_core::time::Timestamp);
    let (mut terminal, mut term) = Term::enter()?;
    let result = event_loop(&mut terminal, &mut term, &mut l);
    // Losing the visit time costs only a longer "merged since" list next time.
    let _ = fs::write(visit, pullpen_core::time::Timestamp::now().0.to_string());
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

fn event_loop(terminal: &mut DefaultTerminal, term: &mut Term, l: &mut Loop) -> Result<()> {
    // Debug builds only: lets the panic path be exercised by hand
    // (`PULLPEN_TEST_PANIC=1 cargo run -- --fixtures`) without shipping a
    // crash switch in release binaries.
    #[cfg(debug_assertions)]
    if std::env::var_os("PULLPEN_TEST_PANIC").is_some() {
        draw(terminal, l.app)?;
        panic!("PULLPEN_TEST_PANIC is set");
    }
    while !l.app.quit {
        l.app.sync = l.backend.sync_label();
        let title = l.app.title();
        if title != term.title {
            execute!(stdout(), SetTitle(&title))?;
            term.title = title;
        }
        draw(terminal, l.app)?;
        let mut effects = Vec::new();
        if event::poll(TICK)? {
            match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => {
                    if let Some(key) = keys::convert(k) {
                        // Remaps apply only where a key is an action, not inside
                        // the command line, filter or help, where it is text.
                        let typing = !matches!(l.app.input, pullpen_ui::app::Input::Normal);
                        let key = if typing {
                            key
                        } else {
                            l.remap.iter().find(|(from, _)| *from == key).map_or(key, |(_, to)| *to)
                        };
                        effects.extend(l.app.key(key, Instant::now()));
                    }
                }
                Event::FocusGained if l.on_focus => l.backend.refresh(None),
                Event::Mouse(m) => {
                    let (x, y) = (m.column, m.row);
                    let ev = match m.kind {
                        MouseEventKind::Down(MouseButton::Left) => Some(Mouse::Click { x, y }),
                        MouseEventKind::ScrollDown => Some(Mouse::Scroll { x, down: true }),
                        MouseEventKind::ScrollUp => Some(Mouse::Scroll { x, down: false }),
                        _ => None,
                    };
                    if let Some(ev) = ev {
                        l.app.mouse(ev, Instant::now());
                    }
                }
                // A resize needs nothing beyond the redraw at the top of the
                // loop: ratatui re-reads the size on every draw.
                _ => {}
            }
        } else {
            l.app.expire(Instant::now());
            if !l.app.jobs.is_empty() {
                l.app.tick += 1;
            }
        }
        for m in l.backend.messages() {
            match m {
                Msg::Job(r) => effects.extend(l.app.job_done(r)),
                Msg::Synced { section, prs, total } => l.app.synced(section, prs, total),
                Msg::Down(f, msg) => l.app.forge_down(f, msg),
                Msg::Up(f) => l.app.forge_up(f),
                Msg::Caps(c) => l.app.caps.extend(c),
                Msg::Detail(id, d) => l.app.detail_loaded(id, d),
                Msg::Diff(id, f) => l.app.diff_loaded(id, f),
                Msg::Runs(id, runs) => l.app.runs_loaded(id, runs),
                Msg::Log(id, job, raw) => l.app.log_loaded(id, job, &raw),
                Msg::Unwatched { log, why } => {
                    tracing::warn!(log, %why, "pipeline fetch failed");
                    l.app.watch_failed(log, &why);
                }
                Msg::Unfetched { id, diff, why } => {
                    tracing::warn!(pr = id.number, diff, %why, "fetch failed");
                    l.app.fetch_failed(&id, diff);
                }
            }
        }
        effects.extend(l.app.wants());
        effects.extend(l.app.watch(Instant::now()));
        for e in effects {
            run_effect(e, terminal, term, l)?;
        }
    }
    Ok(())
}

fn run_effect(e: Effect, terminal: &mut DefaultTerminal, term: &mut Term, l: &mut Loop) -> Result<()> {
    match e {
        Effect::Run(job) => l.backend.run(job),
        Effect::Edit { targets } => {
            term.leave();
            let text = edit(l.dirs, &targets);
            let (t, k) = Term::enter()?;
            *terminal = t;
            *term = k;
            terminal.clear()?;
            match text {
                Ok(text) => {
                    for e in l.app.comment_text(targets, text) {
                        if let Effect::Run(job) = e {
                            l.backend.run(job);
                        }
                    }
                }
                Err(err) => l.app.error(&format!("{err:#}"), Mode::Normal),
            }
        }
        Effect::Open(url) => {
            if let Err(why) = l.backend.open(&url) {
                l.app.error(&why, Mode::Normal);
            }
        }
        Effect::Refresh { all } => l.backend.refresh(if all { None } else { Some(l.app.active) }),
        Effect::FetchDetail(id) => l.backend.fetch(id, false),
        Effect::FetchDiff(id) => l.backend.fetch(id, true),
        Effect::FetchRuns(id) => l.backend.runs(id),
        Effect::FetchLog(id, job) => l.backend.log(id, job),
        Effect::Notify(text) => {
            // The bell, then OSC 9, which terminals that support it turn into
            // a desktop notification (design §06).
            let mut out = stdout();
            if l.bell {
                write!(out, "\x07")?;
            }
            if l.osc9 {
                write!(out, "\x1b]9;pullpen: {}\x07", text.replace(['\x07', '\x1b'], ""))?;
            }
            out.flush()?;
        }
        Effect::Redraw => terminal.clear()?,
        Effect::SaveHistory => save_history(&l.dirs.state.join("history"), &l.app.history)?,
        Effect::SaveCleared => {
            let path = l.dirs.state.join("cleared.json");
            fs::write(&path, serde_json::to_string(&l.app.cleared)?)
                .with_context(|| format!("writing {}", path.display()))?
        }
    }
    Ok(())
}

/// Opens the editor (see `editor`) on a scratch file in the cache
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
    let editor = editor();
    let mut parts = editor.split_whitespace();
    let bin = parts.next().unwrap_or(FALLBACK_EDITOR);
    let status = Command::new(bin).args(parts).arg(&path).status().with_context(|| format!("starting {editor}"))?;
    anyhow::ensure!(status.success(), "{editor} exited with {status}; nothing posted");
    let text = fs::read_to_string(&path)?;
    // Removing the scratch file is tidiness; a leftover is overwritten next time.
    let _ = fs::remove_file(&path);
    Ok(text.lines().filter(|l| !l.starts_with('#')).collect::<Vec<_>>().join("\n").trim().to_string())
}

/// Windows has no vi; Notepad is the editor every Windows machine has.
const FALLBACK_EDITOR: &str = if cfg!(windows) { "notepad" } else { "vi" };

/// $VISUAL, then $EDITOR, then the platform's fallback.
pub fn editor() -> String {
    std::env::var("VISUAL").or_else(|_| std::env::var("EDITOR")).unwrap_or_else(|_| FALLBACK_EDITOR.into())
}

fn load_history(path: &Path) -> Vec<String> {
    fs::read_to_string(path).map(|t| t.lines().map(String::from).collect()).unwrap_or_default()
}

fn save_history(path: &Path, h: &[String]) -> Result<()> {
    let keep = &h[h.len().saturating_sub(HISTORY_MAX)..];
    fs::write(path, keep.join("\n") + "\n").with_context(|| format!("writing {}", path.display()))
}
