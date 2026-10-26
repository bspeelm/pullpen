//! The renderer against the design, byte for byte (ADR-002, PLAN.md §7).
//! Each case transcribes the state tui.js `build()` gives that screen.

// A test that cannot read its inputs should fail loudly, which is what
// unwrap does; clippy's test allowance does not reach helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;

use pullpen_core::{Forge, Pr};
use pullpen_fixtures as fx;
use pullpen_ui::glyphs::Glyphs;
use pullpen_ui::paint::{Theme, paint};
use pullpen_ui::palette::{Palette, Tier};
use pullpen_ui::snapshot::serialize;
use pullpen_ui::span::{Line, lr, markup};
use pullpen_ui::views::Ctx;
use pullpen_ui::views::chrome::{Mode, Status, Tab};
use pullpen_ui::views::list::{ListScreen, PREVIEW_W, RowOpts, list_screen, strip};
use pullpen_ui::views::preview::preview;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

const FORGES: [Forge; 2] = [Forge::GitHub, Forge::GitLab];

fn tabs() -> Vec<Tab> {
    fx::sections()
        .into_iter()
        .map(|s| Tab {
            name: s.section.name,
            short: s.section.short,
            count: s.total,
        })
        .collect()
}

fn status(mode: Mode, section: &str, pos: &str) -> Status<'static> {
    Status {
        mode,
        section: Box::leak(section.to_string().into_boxed_str()),
        pos: pos.into(),
        extra: vec![],
        armed: 7,
        failing: 3,
        narrow: false,
        err: None,
        sync: "synced 40s".into(),
        forges: &FORGES,
    }
}

fn check(name: &str, lines: &[Line], w: u16, tier: Tier, glyphs: Glyphs) {
    let h = lines.len() as u16;
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    let theme = Theme {
        palette: Palette::new(tier),
        glyphs,
    };
    let area = buf.area;
    paint(lines, &mut buf, area, &theme);
    let (plain, styled) = serialize(&buf, tier);
    let (gp, gs) = fx::golden(name);
    if plain != gp {
        panic!(
            "{name}: text differs from the design\n{}",
            first_diff(&gp, &plain)
        );
    }
    if styled != gs {
        panic!(
            "{name}: style differs from the design\n{}",
            first_diff(&gs, &styled)
        );
    }
}

fn first_diff(want: &str, got: &str) -> String {
    for (i, (w, g)) in want.lines().zip(got.lines()).enumerate() {
        if w != g {
            return format!("row {i}\n want: {w}\n  got: {g}");
        }
    }
    format!(
        "row counts differ: want {}, got {}",
        want.lines().count(),
        got.lines().count()
    )
}

fn ctx(caps: &HashMap<String, pullpen_core::Capabilities>, glyphs: Glyphs) -> Ctx<'_> {
    Ctx {
        now: fx::NOW,
        glyphs,
        caps,
    }
}

#[test]
fn s_list() {
    let caps = HashMap::new();
    let c = ctx(&caps, Glyphs::Unicode);
    let prs = fx::agent_prs();
    let pv = preview(&prs[0], &fx::detail(4821), PREVIEW_W, &c);
    let t = tabs();
    let s = ListScreen {
        w: 120,
        h: 40,
        tabs: &t,
        active: 2,
        short: false,
        right: "author:@agents",
        prs: &prs,
        sel: 0,
        offset: 0,
        marks: &[],
        hl: None,
        preview: Some(pv),
        status: status(Mode::Normal, "Agent-opened", "1/23"),
        cmd: vec![],
    };
    check(
        "sList",
        &list_screen(&s, &c),
        120,
        Tier::True,
        Glyphs::Unicode,
    );
}

#[test]
fn s_filter() {
    let caps = HashMap::new();
    let c = ctx(&caps, Glyphs::Unicode);
    let prs = fx::all_prs();
    let sel = prs
        .iter()
        .position(|p| p.title.to_lowercase().contains("ledger"))
        .unwrap();
    let t = tabs();
    let s = ListScreen {
        w: 120,
        h: 40,
        tabs: &t,
        active: 4,
        short: false,
        right: "is:open   sort: updated",
        prs: &prs,
        sel,
        offset: 0,
        marks: &[],
        hl: Some("ledger"),
        preview: None,
        status: status(Mode::Filter, "All", "1/5 matches"),
        cmd: lr(
            &markup("/ledger{C| }"),
            &markup("{d|n N jump   Enter hide non-matches   Esc clear}"),
            120,
        ),
    };
    check(
        "sFilter",
        &list_screen(&s, &c),
        120,
        Tier::True,
        Glyphs::Unicode,
    );
}

fn s80(glyphs: Glyphs) -> Vec<Line> {
    let caps = HashMap::new();
    let c = ctx(&caps, glyphs);
    let prs = fx::agent_prs();
    let t = tabs();
    let mut st = status(Mode::Normal, "Agent", "1/23");
    st.narrow = true;
    let s = ListScreen {
        w: 80,
        h: 24,
        tabs: &t,
        active: 2,
        short: true,
        right: "",
        prs: &prs,
        sel: 0,
        offset: 0,
        marks: &[],
        hl: None,
        preview: None,
        status: st,
        cmd: vec![],
    };
    list_screen(&s, &c)
}

#[test]
fn s_80() {
    check(
        "s80",
        &s80(Glyphs::Unicode),
        80,
        Tier::True,
        Glyphs::Unicode,
    );
}

#[test]
fn s_80_ascii_sixteen_colours() {
    check(
        "s80ascii",
        &s80(Glyphs::Ascii),
        80,
        Tier::C16,
        Glyphs::Ascii,
    );
}

/// tui.js `strip`: rows, then status and command line.
fn strip_of(prs: &[Pr], sel: usize, st: Status, cmd: Line) -> Vec<Line> {
    let caps = HashMap::new();
    let c = ctx(&caps, Glyphs::Unicode);
    let opts: Vec<RowOpts> = (0..prs.len())
        .map(|i| RowOpts {
            sel: i == sel,
            ..Default::default()
        })
        .collect();
    strip(prs, &opts, 120, &st, &cmd, &c)
}

#[test]
fn cmd_idle() {
    let prs: Vec<Pr> = fx::agent_prs().into_iter().take(5).collect();
    let lines = strip_of(&prs, 1, status(Mode::Normal, "Agent-opened", ""), vec![]);
    check("cmd-0", &lines, 120, Tier::True, Glyphs::Unicode);
}
