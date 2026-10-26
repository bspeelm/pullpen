//! The `?` overlay (tui.js `helpScreen`), generated from the keymap table.

use crate::keymap::{ENTRIES, Entry, Group, keys_for};
use crate::palette::Tok;
use crate::span::{Line, fit, lr, markup, overlay, s, sp, with_bg};

pub const PANEL_W: usize = 96;
const COL_W: usize = 30;
pub const TOP: usize = 5;

/// Everything behind the overlay goes faint and loses its fills, except the
/// status and command lines, which stay readable.
pub fn fade(lines: &mut [Line], keep_from: usize) {
    for l in lines.iter_mut().take(keep_from) {
        for sp in l.iter_mut() {
            sp.fg = Tok::Faint;
            sp.bg = None;
            sp.bold = false;
            sp.underline = false;
        }
    }
}

fn pad(l: &[crate::span::Span]) -> Line {
    let mut v = vec![sp(3)];
    v.extend_from_slice(l);
    with_bg(fit(&v, PANEL_W, true), Tok::Surf)
}

fn key_cell(e: Option<&&Entry>, sections: usize) -> Line {
    match e {
        Some(e) => {
            let mut l = fit(&[s(keys_for(e, sections), Tok::Fg).bold()], 11, true);
            l.extend(fit(&[s(e.what, Tok::Dim)], COL_W - 11, true));
            l
        }
        None => vec![sp(COL_W)],
    }
}

fn matches(e: &Entry, filter: &str) -> bool {
    let f = filter.to_lowercase();
    f.is_empty() || e.keys.to_lowercase().contains(&f) || e.what.to_lowercase().contains(&f)
}

/// The panel's rows. `filter` narrows the entries as the user types.
pub fn panel(sections: usize, filter: &str) -> Vec<Line> {
    let title = if filter.is_empty() { markup("{B|keys}") } else { markup(&format!("{{B|keys}}{{a|  {filter}}}")) };
    let mut out = vec![pad(&[]), pad(&lr(&title, &markup("{d|? or Esc to close}"), PANEL_W - 6)), pad(&[])];
    for block in Group::OVERLAY {
        let cols: Vec<Vec<&Entry>> =
            block.iter().map(|g| ENTRIES.iter().filter(|e| e.group == *g && matches(e, filter)).collect()).collect();
        let heads: Line = block.iter().flat_map(|g| fit(&[s(g.heading(), Tok::Acc)], COL_W, true)).collect();
        out.push(pad(&heads));
        let n = cols.iter().map(Vec::len).max().unwrap_or(0);
        for i in 0..n {
            let row: Line = cols.iter().flat_map(|c| key_cell(c.get(i), sections)).collect();
            out.push(pad(&row));
        }
        out.push(pad(&[]));
    }
    out.push(pad(&markup("{d|Every key has a : command. M and x confirm on the command line, never in a dialog.}")));
    out.push(pad(&[]));
    out
}

/// Lays the panel over `base` (already faded), centred, from row `TOP`.
pub fn place(base: &mut [Line], panel: &[Line], w: usize) {
    let x = w.saturating_sub(PANEL_W) / 2;
    for (i, l) in panel.iter().enumerate() {
        if let Some(row) = base.get_mut(TOP + i) {
            *row = fit(&overlay(row, x, l), w, false);
        }
    }
}
