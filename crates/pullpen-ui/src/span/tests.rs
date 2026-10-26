use super::*;
use crate::palette::Tok;

fn text(l: &[Span]) -> String {
    l.iter().map(|s| s.text.as_str()).collect()
}

#[test]
fn fit_pads_short_lines_with_plain_spaces() {
    let l = fit(&[s("ab", Tok::Dim)], 5, true);
    assert_eq!(text(&l), "ab   ");
    assert_eq!(l[1].fg, Tok::Fg);
}

#[test]
fn fit_truncates_with_an_ellipsis_in_the_cut_spans_style() {
    let l = fit(&[s("abc", Tok::Fg), s("defgh", Tok::Dim).underline()], 6, true);
    assert_eq!(text(&l), "abcde…");
    let last = l.last().unwrap();
    assert_eq!((last.fg, last.underline), (Tok::Dim, false));
}

#[test]
fn fit_moves_trailing_spaces_after_the_ellipsis() {
    // tui.js: a cut that lands in trailing padding keeps the padding width.
    let l = fit(&[s("ab   ", Tok::Dim), s("zz", Tok::Fg)], 6, true);
    assert_eq!(text(&l), "ab…   ");
}

#[test]
fn fit_without_ellipsis_cuts_hard() {
    assert_eq!(text(&fit(&[s("abcdef", Tok::Fg)], 4, false)), "abcd");
}

#[test]
fn fit_to_zero_is_empty() {
    assert!(fit(&[s("abc", Tok::Fg)], 0, true).is_empty());
}

#[test]
fn lr_right_justifies_and_truncates_the_left() {
    assert_eq!(text(&lr(&[s("left", Tok::Fg)], &[s("R", Tok::Fg)], 8)), "left   R");
    assert_eq!(text(&lr(&[s("leftmost", Tok::Fg)], &[s("R", Tok::Fg)], 6)), "left…R");
}

#[test]
fn widths_count_chars_not_bytes() {
    assert_eq!(width(&[s("◆ ○ ✓ −", Tok::Fg)]), 7);
}

#[test]
fn markup_parses_tags_and_plain_text() {
    let l = markup("{g|✓} {d|checks}{aB|x}");
    assert_eq!(text(&l), "✓ checksx");
    assert_eq!(l[0].fg, Tok::Ok);
    assert_eq!(l[1].fg, Tok::Fg);
    assert_eq!((l[3].fg, l[3].bold), (Tok::Acc, true));
}

#[test]
fn markup_cursor_is_fg_on_bg_inverted() {
    let l = markup("/x{C| }");
    assert_eq!((l[1].fg, l[1].bg), (Tok::Bg, Some(Tok::Fg)));
}

#[test]
fn a_brace_that_opens_no_tag_is_plain_text() {
    assert_eq!(text(&markup("a {b} {d|c}")), "a {b} c");
}

#[test]
fn overlay_replaces_cells_and_remerges() {
    let l = overlay(&[s("aaaaaa", Tok::Fg)], 2, &[s("bb", Tok::Fg)]);
    assert_eq!(text(&l), "aabbaa");
    assert_eq!(l.len(), 1);
}

#[test]
fn wrap_is_greedy_and_never_splits_words() {
    assert_eq!(wrap("aa bb cc dddddd", 5), vec!["aa bb", "cc", "dddddd"]);
}
