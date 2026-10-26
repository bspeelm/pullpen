//! `pullpen settings`: every setting by name, with its value, changed one at
//! a time and saved at once (PLAN.md §6.6). `get` and `set` do the same from
//! a script. Sections and key remaps are lists, so they open in $EDITOR.

use std::io;

use crate::config::{Config, Kind};
use crate::prompt::Io;

/// Each setting: its name, what it means, and the values it takes.
pub const KEYS: &[(&str, &str, &str)] = &[
    ("merge.default_method", "how to merge when you do not say", "repo, squash, merge, rebase"),
    ("merge.delete_branch", "delete the branch after merging", "yes, no"),
    ("notify.bell", "ring when an armed PR merges or needs you", "yes, no"),
    ("notify.osc9", "send a desktop notification as well", "yes, no"),
    ("sync.interval", "how often each section refreshes", "60s, 5m, …"),
    ("sync.on_focus", "refresh when the terminal regains focus", "yes, no"),
    ("ui.glyphs", "glyph set", "unicode, ascii, nerd"),
    ("ui.colors", "colour depth", "auto, 16, 256, true"),
    ("ui.preview", "the preview pane", "auto, on, off"),
    ("ui.accent", "accent colour", "#rrggbb, or blank for the default"),
    ("ui.palette", "a palette file of your own", "a path, or blank"),
    ("github.scope", "orgs or repos an unscoped GitHub list searches", "space-separated, blank: PRs that involve you"),
    ("gitlab.groups", "groups an unscoped GitLab list searches", "space-separated"),
    ("checkout.worktree", ":checkout makes a worktree", "yes, no"),
];

fn yn(b: bool) -> String {
    if b { "yes".into() } else { "no".into() }
}

fn parse_bool(v: &str) -> Result<bool, String> {
    match v.to_lowercase().as_str() {
        "yes" | "y" | "true" | "on" => Ok(true),
        "no" | "n" | "false" | "off" => Ok(false),
        _ => Err(format!("{v:?} is not yes or no")),
    }
}

fn one_of(v: &str, allowed: &[&str]) -> Result<String, String> {
    allowed
        .iter()
        .find(|a| **a == v)
        .map(|a| a.to_string())
        .ok_or_else(|| format!("{v:?} is not one of {}", allowed.join(", ")))
}

pub fn get(c: &Config, key: &str) -> Result<String, String> {
    let forge = |k: Kind| c.forge.iter().find(|f| f.kind == k);
    Ok(match key {
        "merge.default_method" => c.merge.default_method.clone(),
        "merge.delete_branch" => yn(c.merge.delete_branch),
        "notify.bell" => yn(c.notify.bell),
        "notify.osc9" => yn(c.notify.osc9),
        "sync.interval" => c.sync.interval.clone(),
        "sync.on_focus" => yn(c.sync.on_focus),
        "ui.glyphs" => c.ui.glyphs.clone(),
        "ui.colors" => c.ui.colors.clone(),
        "ui.preview" => c.ui.preview.clone(),
        "ui.accent" => c.ui.accent.clone().unwrap_or_default(),
        "ui.palette" => c.ui.palette.clone().unwrap_or_default(),
        "github.scope" => forge(Kind::Github).and_then(|f| f.scope.clone()).unwrap_or_default(),
        "gitlab.groups" => forge(Kind::Gitlab).map(|f| f.groups.join(" ")).unwrap_or_default(),
        "checkout.worktree" => yn(c.checkout.worktree),
        _ => return Err(format!("no setting called {key}; `pullpen settings` lists them")),
    })
}

/// Checks a value before taking it: a setting is never saved in a shape
/// pullpen cannot read back.
pub fn set(c: &mut Config, key: &str, v: &str) -> Result<(), String> {
    let v = v.trim();
    let opt = |v: &str| (!v.is_empty()).then(|| v.to_string());
    match key {
        "merge.default_method" => c.merge.default_method = one_of(v, &["repo", "squash", "merge", "rebase"])?,
        "merge.delete_branch" => c.merge.delete_branch = parse_bool(v)?,
        "notify.bell" => c.notify.bell = parse_bool(v)?,
        "notify.osc9" => c.notify.osc9 = parse_bool(v)?,
        "sync.on_focus" => c.sync.on_focus = parse_bool(v)?,
        "checkout.worktree" => c.checkout.worktree = parse_bool(v)?,
        "sync.interval" => {
            let ok = v.trim_end_matches(['s', 'm']).parse::<u64>().is_ok() && !v.is_empty();
            if !ok {
                return Err(format!("{v:?} is not a duration like 60s or 5m"));
            }
            c.sync.interval = v.into();
        }
        "ui.glyphs" => c.ui.glyphs = one_of(v, &["unicode", "ascii", "nerd"])?,
        "ui.colors" => c.ui.colors = one_of(v, &["auto", "16", "256", "true"])?,
        "ui.preview" => c.ui.preview = one_of(v, &["auto", "on", "off"])?,
        "ui.accent" => {
            if !v.is_empty() && pullpen_ui::palette::parse_hex(v).is_none() {
                return Err(format!("{v:?} is not a #rrggbb colour"));
            }
            c.ui.accent = opt(v);
        }
        "ui.palette" => c.ui.palette = opt(v),
        "github.scope" | "gitlab.groups" => {
            let kind = if key == "github.scope" { Kind::Github } else { Kind::Gitlab };
            let f = c.forge.iter_mut().find(|f| f.kind == kind).ok_or_else(|| {
                format!("no {} forge yet; `pullpen setup` adds one", key.split('.').next().unwrap_or(""))
            })?;
            if kind == Kind::Github {
                let terms: Vec<String> = v
                    .split_whitespace()
                    .map(|t| {
                        if t.contains(':') {
                            t.into()
                        } else if t.contains('/') {
                            format!("repo:{t}")
                        } else {
                            format!("org:{t}")
                        }
                    })
                    .collect();
                f.scope = (!terms.is_empty()).then(|| terms.join(" "));
            } else {
                f.groups = v.split_whitespace().map(String::from).collect();
            }
        }
        _ => return Err(format!("no setting called {key}; `pullpen settings` lists them")),
    }
    Ok(())
}

/// What the menu asks the caller to do besides saving.
pub enum Leave {
    Done,
    Setup,
    Edit,
}

/// The menu. `save` is called after every change, so leaving with Ctrl-C
/// loses nothing already answered.
pub fn menu(
    io: &mut Io,
    c: &mut Config,
    signed_in: &str,
    save: &mut dyn FnMut(&Config) -> io::Result<()>,
) -> io::Result<Leave> {
    loop {
        io.say(&format!("\npullpen settings\n\n   s  sign-in         {signed_in}"))?;
        for (i, (k, _, _)) in KEYS.iter().enumerate() {
            let v = get(c, k).unwrap_or_default();
            io.say(&format!("  {:>2}  {k:<22}{}", i + 1, if v.is_empty() { "—".into() } else { v }))?;
        }
        io.say("   e  sections and key remaps, in $EDITOR\n   q  done\n")?;
        let a = io.line("Change which", "q")?;
        match a.as_str() {
            "q" => return Ok(Leave::Done),
            "s" => return Ok(Leave::Setup),
            "e" => return Ok(Leave::Edit),
            _ => {}
        }
        let Some((key, what, values)) = a.parse::<usize>().ok().and_then(|i| KEYS.get(i.wrapping_sub(1))) else {
            io.say("  a number from the list, or s, e or q")?;
            continue;
        };
        io.say(&format!("\n  {key}: {what}\n  takes: {values}"))?;
        let now = get(c, key).unwrap_or_default();
        let v = io.line("  new value", &now)?;
        match set(c, key, &v) {
            Ok(()) => {
                save(c)?;
                io.say(&format!("  ✓ {key} = {}", get(c, key).unwrap_or_default()))?;
            }
            Err(e) => io.say(&format!("  ✗ {e}; unchanged"))?,
        }
    }
}

#[cfg(test)]
mod tests;
