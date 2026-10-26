use super::*;

fn text(lines: &[Line]) -> Vec<String> {
    lines.iter().map(|l| l.iter().map(|s| s.text.as_str()).collect::<String>().trim_end().to_string()).collect()
}

#[test]
fn plain_prose_wraps_exactly_as_before() {
    let p = "Moves invoice writes onto the ledger v2 double-entry tables. The backfill runs as an online migration.";
    let old: Vec<String> = crate::span::wrap(p, 30);
    assert_eq!(text(&render(p, 30)), old);
}

#[test]
fn the_syntax_goes_and_the_meaning_stays() {
    let md = "## Summary\n\nAdds **idempotency** keys via `Idempotency-Key`, see [the RFC](https://x.test).\n\n- [x] tests\n- [ ] docs\n1. first\n\n> quoted\n\n```\nlet x = 1;\n```\n\n<!-- template comment -->\n---";
    let out = render(md, 60);
    assert_eq!(
        text(&out),
        [
            "Summary",
            "",
            "Adds idempotency keys via Idempotency-Key, see the RFC.",
            "",
            "✓ tests",
            "○ docs",
            "1. first",
            "",
            "│ quoted",
            "",
            "  let x = 1;",
            "",
            "────────────────────────────────────────",
        ]
    );
    assert!(out[0][0].bold, "a heading is bold");
    assert!(out[2].iter().any(|s| s.text == "idempotency" && s.bold));
}

#[test]
fn a_long_list_item_hangs_under_its_text() {
    let out = render("- one two three four five six", 12);
    assert_eq!(text(&out), ["- one two", "  three four", "  five six"]);
}

#[test]
fn snake_case_and_comparisons_are_left_alone() {
    let out = render("set max_retry_count when a < b and b > c", 80);
    assert_eq!(text(&out), ["set max_retry_count when a < b and b > c"]);
}

#[test]
fn the_preview_takes_the_first_prose_paragraph() {
    assert_eq!(first_paragraph("## Summary\n\nFixes **the** bug.\n\nMore."), "Fixes the bug.");
}
