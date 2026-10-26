//! Terminal lifecycle and the event loop.
//!
//! `ratatui::try_init` installs a panic hook that leaves raw mode and the
//! alternate screen before the panic message prints, so a crash never strands
//! the user in a broken terminal. `restore` runs on every normal exit path,
//! errors included.

use std::io::{Write, stdout};

use anyhow::Result;
use crossterm::event::{
    self, DisableFocusChange, EnableFocusChange, Event, KeyCode, KeyEventKind, KeyModifiers,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    BeginSynchronizedUpdate, EndSynchronizedUpdate, supports_keyboard_enhancement,
};
use crossterm::{execute, queue};
use pullpen_core::time::Timestamp;
use pullpen_ui::app::App;
use ratatui::DefaultTerminal;

pub fn run(app: &mut App, now: impl Fn() -> Timestamp) -> Result<()> {
    let mut terminal = ratatui::try_init()?;
    // Where the terminal offers it, the Kitty keyboard protocol makes Tab and
    // ^i (and Esc and Alt-sequences) distinguishable (design §13). Asking a
    // terminal that does not answer is harmless but slow, so ask once.
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
    }
    execute!(stdout(), EnableFocusChange)?;
    let result = event_loop(&mut terminal, app, now);
    // Restoring is best effort: the terminal may already be gone, and an
    // error here would only hide the one `result` carries.
    let _ = execute!(stdout(), DisableFocusChange);
    if kitty {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn draw(terminal: &mut DefaultTerminal, app: &mut App, now: Timestamp) -> Result<()> {
    // Synchronized output (mode 2026): the terminal shows the whole frame at
    // once instead of painting it as it arrives, which is what stops flicker.
    queue!(stdout(), BeginSynchronizedUpdate)?;
    terminal.draw(|f| {
        let area = f.area();
        app.render(area, f.buffer_mut(), now);
    })?;
    queue!(stdout(), EndSynchronizedUpdate)?;
    stdout().flush()?;
    Ok(())
}

fn event_loop(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    now: impl Fn() -> Timestamp,
) -> Result<()> {
    // Debug builds only: lets the panic path be exercised by hand
    // (`PULLPEN_TEST_PANIC=1 cargo run -- --fixtures`) without shipping a
    // crash switch in release binaries.
    #[cfg(debug_assertions)]
    if std::env::var_os("PULLPEN_TEST_PANIC").is_some() {
        draw(terminal, app, now())?;
        panic!("PULLPEN_TEST_PANIC is set");
    }
    while !app.quit {
        draw(terminal, app, now())?;
        match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
                KeyCode::Char('q') => app.quit = true,
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.quit = true
                }
                _ => {}
            },
            // A resize needs no handling beyond the redraw at the top of the
            // loop: ratatui re-reads the size on every draw.
            _ => {}
        }
    }
    Ok(())
}
