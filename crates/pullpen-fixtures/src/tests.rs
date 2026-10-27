use super::*;
use pullpen_core::time;

#[test]
fn all_interleaves_like_tui_js() {
    let all = all_prs();
    assert_eq!(all.len(), 35);
    let head: Vec<u64> = all.iter().take(6).map(|p| p.id.number).collect();
    assert_eq!(head, [4823, 4821, 812, 2291, 2290, 377]);
}

#[test]
fn the_design_counts_hold() {
    let s = sections();
    let totals: Vec<usize> = s.iter().map(|x| x.total).collect();
    assert_eq!(totals, [6, 4, 7, 5, 7, 96], "tui.js SECS");
    assert_eq!(agent_section().total, 23);
}

#[test]
fn opened_ages_render_back_to_the_design_strings() {
    for (raw, pr) in P.iter().zip(agent_prs()) {
        assert_eq!(time::age(pr.created_at, NOW), raw.age, "#{}", raw.n);
    }
}

#[test]
fn the_diff_has_fourteen_files_and_the_design_hunks_second() {
    let d = diff(4821);
    assert_eq!(d.len(), 14);
    assert_eq!((d[1].path.as_str(), d[1].additions, d[1].deletions), ("src/ledger/migrate.ts", 120, 12));
    assert_eq!(d[1].hunks.len(), 2);
    assert_eq!(d[1].hunks[0].lines.len(), 18);
}
