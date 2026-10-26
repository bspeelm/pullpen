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
    assert_eq!(totals, [6, 4, 23, 7, 96]);
}

#[test]
fn ages_render_back_to_the_design_strings() {
    for (raw, pr) in P.iter().zip(agent_prs()) {
        assert_eq!(time::age(pr.updated_at, NOW), raw.age, "#{}", raw.n);
    }
}
