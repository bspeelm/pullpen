use super::*;

const DESIGN: &str = "✓✗○≠◌◆◇●▌│…─•→←›↑↓·−–⠋⠙⠹⠸";

#[test]
fn every_glyph_in_every_tier_is_one_cell() {
    // The CJK width would make ○ ◆ ● ✓ two cells in some locales and shift
    // every column after them (PLAN.md §5.1).
    for tier in [Glyphs::Unicode, Glyphs::Ascii, Glyphs::Nerd] {
        for c in DESIGN.chars() {
            let m = tier.map(c);
            let w = unicode_width::UnicodeWidthChar::width(m);
            assert_eq!(
                w,
                Some(1),
                "{c:?} maps to {m:?} in {tier:?}, which is {w:?} cells"
            );
        }
        for f in [Forge::GitHub, Forge::GitLab] {
            let s = tier.forge(f);
            assert_eq!(
                unicode_width::UnicodeWidthStr::width(s),
                2,
                "{tier:?} forge tag {s:?}"
            );
        }
    }
}

#[test]
fn ascii_spinner_follows_the_glyph_table() {
    // The spec's glyph table (§10) gives the four frames; tui.js shows only
    // one frame and maps it differently, and the table is the spec.
    let frames: String = SPINNER.iter().map(|c| Glyphs::Ascii.map(*c)).collect();
    assert_eq!(frames, "|/-\\");
}
