//! Terminal lifecycle and the event loop.
//!
//! `ratatui::try_init` installs a panic hook that leaves raw mode and the
//! alternate screen before the panic message prints, so a crash never strands
//! the user in a broken terminal. `restore` runs on every normal exit path,
//! errors included.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::cli::Args;

pub fn run(args: &Args) -> Result<()> {
    let mut terminal = ratatui::try_init()?;
    let result = event_loop(&mut terminal, args);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, _args: &Args) -> Result<()> {
    // Debug builds only: lets the panic path be exercised by hand
    // (`PULLPEN_TEST_PANIC=1 cargo run -- --fixtures`) without shipping a
    // crash switch in release binaries.
    #[cfg(debug_assertions)]
    if std::env::var_os("PULLPEN_TEST_PANIC").is_some() {
        terminal.draw(|_| {})?;
        panic!("PULLPEN_TEST_PANIC is set");
    }
    loop {
        terminal.draw(|_| {})?;
        match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                _ => {}
            },
            // A resize needs no handling beyond the redraw at the top of the
            // loop: ratatui re-reads the size on every draw.
            _ => {}
        }
    }
}
