//! Questions on the terminal, for `pullpen setup` and `pullpen settings`.
//! Plain lines rather than the TUI: these run before there is anything to
//! draw, and they must work over SSH and in a script.

use std::io::{self, BufRead, Write};

/// Where answers come from and questions go. Tests give it a script.
pub struct Io<'a> {
    pub input: &'a mut dyn BufRead,
    pub out: &'a mut dyn Write,
    /// Reads a line without echoing it, for tokens.
    pub secret: &'a mut dyn FnMut() -> io::Result<String>,
}

impl Io<'_> {
    pub fn say(&mut self, text: &str) -> io::Result<()> {
        writeln!(self.out, "{text}")
    }

    /// A line, or `default` for an empty answer. End of input is an error,
    /// so a script that runs out does not loop on the same question.
    pub fn line(&mut self, q: &str, default: &str) -> io::Result<String> {
        if default.is_empty() {
            write!(self.out, "{q}: ")?
        } else {
            write!(self.out, "{q} [{default}]: ")?
        }
        self.out.flush()?;
        let mut s = String::new();
        if self.input.read_line(&mut s)? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "no more answers"));
        }
        let s = s.trim();
        Ok(if s.is_empty() { default.to_string() } else { s.to_string() })
    }

    pub fn yes(&mut self, q: &str, default: bool) -> io::Result<bool> {
        loop {
            let a = self.line(&format!("{q} ({})", if default { "Y/n" } else { "y/N" }), "")?.to_lowercase();
            match a.as_str() {
                "" => return Ok(default),
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => self.say("  y or n")?,
            }
        }
    }

    /// One of `options` by number or by name; returns its index.
    pub fn pick(&mut self, q: &str, options: &[&str], default: usize) -> io::Result<usize> {
        for (i, o) in options.iter().enumerate() {
            self.say(&format!("  {}  {o}", i + 1))?;
        }
        loop {
            let a = self.line(q, options[default])?;
            if let Some(i) = a.parse::<usize>().ok().filter(|i| (1..=options.len()).contains(i)) {
                return Ok(i - 1);
            }
            if let Some(i) = options.iter().position(|o| o.eq_ignore_ascii_case(&a)) {
                return Ok(i);
            }
            self.say(&format!("  a number from 1 to {}", options.len()))?;
        }
    }

    pub fn hidden(&mut self, q: &str) -> io::Result<String> {
        write!(self.out, "{q} (hidden): ")?;
        self.out.flush()?;
        let s = (self.secret)()?;
        writeln!(self.out)?;
        Ok(s.trim().to_string())
    }
}

/// Reads a line from the terminal with echo off, so a pasted token never
/// appears on screen or in the scrollback.
pub fn read_hidden() -> io::Result<String> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
    crossterm::terminal::enable_raw_mode()?;
    let mut s = String::new();
    let result = loop {
        match event::read() {
            Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => match k.code {
                KeyCode::Enter => break Ok(()),
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
                }
                KeyCode::Backspace => {
                    s.pop();
                }
                KeyCode::Char(c) => s.push(c),
                _ => {}
            },
            Ok(Event::Paste(p)) => s.push_str(&p),
            Ok(_) => {}
            Err(e) => break Err(e),
        }
    };
    crossterm::terminal::disable_raw_mode()?;
    result.map(|()| s)
}

#[cfg(test)]
mod tests;
