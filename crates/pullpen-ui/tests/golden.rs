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
    fx::sections().into_iter().map(|s| Tab { name: s.section.name, short: s.section.short, count: s.total }).collect()
}

fn status(mode: Mode, section: &str, pos: &str) -> Status<'static> {
    Status {
        mode,
        section: section.to_string(),
        pos: pos.into(),
        extra: vec![],
        armed: 7,
        failing: 3,
        broken: 1,
        narrow: false,
        err: None,
        sync: "synced 40s".into(),
        forges: &FORGES,
    }
}

fn check(name: &str, lines: &[Line], w: u16, tier: Tier, glyphs: Glyphs) {
    let h = lines.len() as u16;
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    let theme = Theme { palette: Palette::new(tier), glyphs };
    let area = buf.area;
    paint(lines, &mut buf, area, &theme);
    let (plain, styled) = serialize(&buf, tier);
    let (gp, gs) = fx::golden(name);
    if plain != gp {
        panic!("{name}: text differs from the design\n{}", first_diff(&gp, &plain));
    }
    if styled != gs {
        panic!("{name}: style differs from the design\n{}", first_diff(&gs, &styled));
    }
}

fn first_diff(want: &str, got: &str) -> String {
    for (i, (w, g)) in want.lines().zip(got.lines()).enumerate() {
        if w != g {
            return format!("row {i}\n want: {w}\n  got: {g}");
        }
    }
    format!("row counts differ: want {}, got {}", want.lines().count(), got.lines().count())
}

fn opts<'a>(prs: &[Pr], sel: usize, hl: Option<&'a str>) -> Vec<RowOpts<'a>> {
    prs.iter()
        .enumerate()
        .map(|(i, p)| {
            let dim_all = hl.is_some_and(|h| !p.title.to_lowercase().contains(h));
            RowOpts { sel: i == sel, hl, dim_all, ..Default::default() }
        })
        .collect()
}

fn ctx(caps: &HashMap<String, pullpen_core::Capabilities>, glyphs: Glyphs) -> Ctx<'_> {
    Ctx { now: fx::NOW, glyphs, caps }
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
        opts: &opts(&prs, 0, None),
        offset: 0,
        preview: Some(pv),
        status: status(Mode::Normal, "Agent-opened", "1/23"),
        cmd: vec![],
    };
    check("sList", &list_screen(&s, &c), 120, Tier::True, Glyphs::Unicode);
}

#[test]
fn s_filter() {
    let caps = HashMap::new();
    let c = ctx(&caps, Glyphs::Unicode);
    let prs = fx::all_prs();
    let sel = prs.iter().position(|p| p.title.to_lowercase().contains("ledger")).unwrap();
    let t = tabs();
    let s = ListScreen {
        w: 120,
        h: 40,
        tabs: &t,
        active: 4,
        short: false,
        right: "is:open   sort: updated",
        prs: &prs,
        opts: &opts(&prs, sel, Some("ledger")),
        offset: 0,
        preview: None,
        status: status(Mode::Filter, "All", "1/5 matches"),
        cmd: lr(&markup("/ledger{C| }"), &markup("{d|n N jump   Enter hide non-matches   Esc clear}"), 120),
    };
    check("sFilter", &list_screen(&s, &c), 120, Tier::True, Glyphs::Unicode);
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
        opts: &opts(&prs, 0, None),
        offset: 0,
        preview: None,
        status: st,
        cmd: vec![],
    };
    list_screen(&s, &c)
}

#[test]
fn s_80() {
    check("s80", &s80(Glyphs::Unicode), 80, Tier::True, Glyphs::Unicode);
}

#[test]
fn s_80_ascii_sixteen_colours() {
    check("s80ascii", &s80(Glyphs::Ascii), 80, Tier::C16, Glyphs::Ascii);
}

/// tui.js `strip`: rows, then status and command line.
fn strip_of(prs: &[Pr], sel: usize, st: Status, cmd: Line) -> Vec<Line> {
    let caps = HashMap::new();
    let c = ctx(&caps, Glyphs::Unicode);
    let opts: Vec<RowOpts> = (0..prs.len()).map(|i| RowOpts { sel: i == sel, ..Default::default() }).collect();
    strip(prs, &opts, 120, &st, &cmd, &c)
}

#[test]
fn cmd_idle() {
    let prs: Vec<Pr> = fx::agent_prs().into_iter().take(5).collect();
    let lines = strip_of(&prs, 1, status(Mode::Normal, "Agent-opened", ""), vec![]);
    check("cmd-0", &lines, 120, Tier::True, Glyphs::Unicode);
}

// ---- the app, driven by keys (M2) ----------------------------------------

use pullpen_core::{JobResult, Outcome};
use pullpen_ui::app::{App, Effect, SectionState};
use pullpen_ui::keymap::Key;
use std::time::Instant;

fn app() -> App {
    let sections = fx::sections().into_iter().map(|s| SectionState::new(s.section, s.prs, s.total)).collect();
    let mut a = App::new(sections, Theme { palette: Palette::new(Tier::True), glyphs: Glyphs::Unicode }, fx::NOW);
    a.active = 2;
    a.sync = "synced 40s".into();
    a.me = "ash".into();
    for s in &a.sections {
        for p in &s.prs {
            a.details.insert(p.id.clone(), fx::detail(p.id.number));
        }
    }
    a
}

fn keys(a: &mut App, ks: &[Key]) -> Vec<Effect> {
    ks.iter().flat_map(|k| a.key(*k, Instant::now())).collect()
}

fn typed(a: &mut App, text: &str) {
    for c in text.chars() {
        a.key(Key::Char(c), Instant::now());
    }
}

fn select(a: &mut App, i: usize) {
    a.section_mut().sel = i;
}

/// Answers every job the effects started, except those on `hold`, and the
/// jobs those answers start in turn.
fn answer(a: &mut App, mut effects: Vec<Effect>, hold: &[u64], fail: Option<(u64, &str)>) {
    while let Some(e) = effects.pop() {
        let Effect::Run(job) = e else { continue };
        if hold.contains(&job.target.number) {
            continue;
        }
        let outcome = match fail {
            Some((n, r)) if n == job.target.number => Outcome::Failed { reason: r.into() },
            _ => Outcome::Done,
        };
        // The fake forge deletes merged branches, as the design's message says.
        let note =
            matches!(job.action, pullpen_core::Action::Merge { .. }).then(|| "source branch deleted".to_string());
        effects.extend(a.job_done(JobResult { job, outcome, note }));
    }
}

fn strip_check(name: &str, a: &App, k: usize, pos: Option<&str>) {
    check(name, &a.strip(k, 120, pos), 120, Tier::True, Glyphs::Unicode);
}

#[test]
fn cmd_typing_resolves_the_target_live() {
    let mut a = app();
    select(&mut a, 1);
    keys(&mut a, &[Key::Char(':')]);
    typed(&mut a, "comment #812 rebased on main, merging after the 2.4 cut");
    strip_check("cmd-1", &a, 5, Some(""));
}

#[test]
fn cmd_completing_a_command() {
    let mut a = app();
    select(&mut a, 1);
    keys(&mut a, &[Key::Char(':'), Key::Char('a'), Key::Tab]);
    strip_check("cmd-2", &a, 5, Some(""));
}

#[test]
fn cmd_completing_a_target() {
    let mut a = app();
    select(&mut a, 1);
    keys(&mut a, &[Key::Char(':')]);
    typed(&mut a, "merge #48");
    keys(&mut a, &[Key::Tab]);
    strip_check("cmd-3", &a, 5, Some(""));
}

#[test]
fn cmd_confirming_a_close() {
    let mut a = app();
    keys(&mut a, &[Key::Char('x')]);
    strip_check("cmd-4", &a, 5, Some(""));
}

#[test]
fn cmd_a_forge_refusal_stays_on_the_command_line() {
    let mut a = app();
    select(&mut a, 2);
    keys(&mut a, &[Key::Char(':')]);
    typed(&mut a, "approve #2290");
    let fx = keys(&mut a, &[Key::Enter]);
    let why = "you are the author of this PR's last commit (repo requires independent review)";
    answer(&mut a, fx, &[], Some((2290, why)));
    strip_check("cmd-5", &a, 5, Some(""));
}

#[test]
fn cmd_typo_suggests() {
    let mut a = app();
    select(&mut a, 1);
    keys(&mut a, &[Key::Char(':')]);
    typed(&mut a, "mrege");
    keys(&mut a, &[Key::Enter]);
    strip_check("cmd-6", &a, 5, Some(""));
}

#[test]
fn cmd_success() {
    let mut a = app();
    select(&mut a, 1);
    let fx = keys(&mut a, &[Key::Char('a')]);
    answer(&mut a, fx, &[], None);
    strip_check("cmd-7", &a, 5, Some(""));
}

#[test]
fn cmd_history_walk() {
    let mut a = app();
    select(&mut a, 1);
    a.history = (0..14).map(|i| format!("approve #{i}")).collect();
    a.history[11] = "comment #812 lgtm, merging after the release".into();
    keys(&mut a, &[Key::Char(':'), Key::Up, Key::Up, Key::Up]);
    strip_check("cmd-8", &a, 5, Some(""));
}

#[test]
fn cmd_background_error_on_the_status_line() {
    let mut a = app();
    select(&mut a, 1);
    a.err = Some((Forge::GitLab, "gitlab.com 502 · retrying in 8s".into()));
    strip_check("cmd-9", &a, 5, Some(""));
}

fn merged_812() -> (App, Vec<Effect>) {
    let mut a = app();
    select(&mut a, 1);
    keys(&mut a, &[Key::Char('M')]);
    let fx = keys(&mut a, &[Key::Char('y')]);
    (a, fx)
}

#[test]
fn merge_flow() {
    let mut a = app();
    select(&mut a, 1);
    keys(&mut a, &[Key::Char('M')]);
    strip_check("mergeFlow-0", &a, 4, Some(""));

    let (mut a, fx) = merged_812();
    a.tick = 2;
    strip_check("mergeFlow-1", &a, 4, Some(""));
    answer(&mut a, fx, &[], None);
    strip_check("mergeFlow-2", &a, 4, None);
}

#[test]
fn merge_refused_when_blocked() {
    let mut a = app();
    select(&mut a, 2);
    keys(&mut a, &[Key::Char('M')]);
    strip_check("mergeFlow-3", &a, 4, Some(""));
}

#[test]
fn merge_offers_to_arm_when_waiting() {
    let mut a = app();
    select(&mut a, 3);
    keys(&mut a, &[Key::Char('M')]);
    strip_check("mergeFlow-4", &a, 4, Some(""));
}

#[test]
fn arm_picks_a_method_then_disarm_toggles() {
    let mut a = app();
    select(&mut a, 3);
    keys(&mut a, &[Key::Char('A')]);
    strip_check("mergeFlow-5", &a, 4, Some(""));
    let fx = keys(&mut a, &[Key::Char('s')]);
    answer(&mut a, fx, &[], None);
    strip_check("mergeFlow-6", &a, 4, Some(""));
    let fx = keys(&mut a, &[Key::Char('A')]);
    answer(&mut a, fx, &[], None);
    strip_check("mergeFlow-7", &a, 4, Some(""));
}

#[test]
fn batch_arm() {
    let mut a = app();
    for i in [1, 3, 9, 11, 12] {
        select(&mut a, i);
        keys(&mut a, &[Key::Char(' ')]);
    }
    select(&mut a, 12);
    keys(&mut a, &[Key::Char('A')]);
    strip_check("batch-0", &a, 13, None);
    let fx = keys(&mut a, &[Key::Char('y')]);
    answer(&mut a, fx, &[812], None);
    a.tick = 2;
    strip_check("batch-1", &a, 13, None);
}

#[test]
fn s_help() {
    let mut a = app();
    keys(&mut a, &[Key::Char('?')]);
    check("sHelp", &a.lines(120, 40), 120, Tier::True, Glyphs::Unicode);
}

#[test]
fn s_detail() {
    let mut a = app();
    let id = a.section().prs[0].id.clone();
    a.diffs.insert(id, fx::diff(4821));
    keys(&mut a, &[Key::Enter, Key::Char(']'), Key::Char('f')]);
    check("sDetail", &a.lines(120, 40), 120, Tier::True, Glyphs::Unicode);
}

/// The design's armed view after coming back: each PR in the state the
/// screen shows, with the times its right-hand column counts from.
fn armed_app() -> App {
    use pullpen_core::time::{Timestamp, parse_age};
    use pullpen_core::{AutoMerge, MergeMethod, MergeTrainPos, PrState};
    let ago = |s: &str| Timestamp(fx::NOW.0 - parse_age(s).unwrap());
    let pr = |n: u64| fx::by_number(n).unwrap();
    let armed = |mut p: Pr, at: &str| {
        p.auto_merge = Some(AutoMerge { method: MergeMethod::Squash, armed_by: "ash".into(), armed_at: ago(at) });
        p
    };
    let landed = fx::merged_prs();
    let merged = |mut p: Pr, at: &str, method| {
        p.auto_merge = None;
        p.state = PrState::Merged { at: ago(at), method };
        p.post_merge = landed.iter().find(|l| l.id == p.id).and_then(|l| l.post_merge.clone());
        p
    };
    let mut p809 = armed(pr(809), "30m");
    p809.checks.eta_mins = Some(4);
    let mut p145 = armed(pr(145), "1h");
    p145.merge_train = Some(MergeTrainPos { position: 2, length: 3 });
    let mut p4809 = armed(pr(4809), "5d");
    (p4809.checks.required_passed, p4809.checks.running, p4809.checks.queued) = (8, 1, vec!["sbom".into()]);
    let rows = vec![
        armed(pr(2290), "2h"),
        armed(pr(4818), "2h"),
        armed(pr(4821), "2h"),
        p809,
        armed(pr(377), "12m"),
        p145,
        p4809,
        merged(pr(2288), "22m", MergeMethod::Squash),
        merged(pr(812), "41m", MergeMethod::Squash),
        merged(pr(1199), "1h", MergeMethod::Squash),
        merged(pr(4815), "1h", MergeMethod::Rebase),
    ];
    let mut a = app();
    a.active = 3;
    a.sections[3].prs = rows;
    a.sections[3].total = 7;
    for (n, since) in [(2290, "18m"), (4818, "25m")] {
        let id = a.sections[3].prs.iter().find(|p| p.id.number == n).unwrap().id.clone();
        a.watches.insert(
            id,
            pullpen_ui::app::Watch {
                group: pullpen_ui::views::armed::Group::Attention,
                since: ago(since),
                disarmed: false,
            },
        );
    }
    // 09:14 UTC on the fixture day, with the clock shown in UTC.
    a.last_visit = Some(Timestamp(fx::NOW.0 - (14 * 3600 + 13 * 60 + 20) + (9 * 3600 + 14 * 60)));
    a
}

#[test]
fn s_armed() {
    check("sArmed", &armed_app().lines(120, 40), 120, Tier::True, Glyphs::Unicode);
}

#[test]
fn s_merged() {
    let mut a = app();
    a.active = 5;
    check("sMerged", &a.lines(120, 40), 120, Tier::True, Glyphs::Unicode);
}

/// The design's pipeline screen: #812 in the Merged section, `w`, and the
/// runs as they were three seconds ago.
fn piped() -> App {
    let mut a = app();
    a.active = 5;
    let at = a.sections[5].prs.iter().position(|p| p.id.number == 812).unwrap();
    a.sections[5].sel = at;
    keys(&mut a, &[Key::Char('w')]);
    let id = a.pipe.as_ref().unwrap().id.clone();
    a.runs_loaded(id, fx::runs(812));
    a.poll.runs_at = Some(pullpen_core::time::Timestamp(fx::NOW.0 - 3));
    a
}

#[test]
fn s_pipeline() {
    check("sPipeline", &piped().lines(120, 40), 120, Tier::True, Glyphs::Unicode);
}

#[test]
fn s_job() {
    let mut a = piped();
    a.height = 40;
    keys(&mut a, &[Key::Enter]);
    let id = a.pipe.as_ref().unwrap().id.clone();
    a.log_loaded(id, "79".into(), fx::LOG);
    check("sJob", &a.lines(120, 40), 120, Tier::True, Glyphs::Unicode);
}
