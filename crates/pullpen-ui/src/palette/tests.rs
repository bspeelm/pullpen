use super::*;

#[test]
fn detection_prefers_colorterm_then_term() {
    assert_eq!(detect(Some("truecolor"), Some("xterm")), Tier::True);
    assert_eq!(detect(Some("24bit"), None), Tier::True);
    assert_eq!(detect(None, Some("xterm-256color")), Tier::C256);
    assert_eq!(detect(None, Some("xterm")), Tier::C16);
    assert_eq!(detect(None, None), Tier::C16);
}

#[test]
fn accent_override_applies_to_truecolor_only() {
    let mut p = Palette::new(Tier::True);
    p.accent = Some((1, 2, 3));
    assert_eq!(p.color(Tok::Acc), Some(Color::Rgb(1, 2, 3)));
    p.tier = Tier::C16;
    assert_eq!(p.color(Tok::Acc), Some(Color::Cyan));
}

#[test]
fn sixteen_colours_never_paint_surface_or_selection() {
    let p = Palette::new(Tier::C16);
    assert_eq!(p.color(Tok::Surf), None);
    assert_eq!(p.color(Tok::Sel), None);
}
