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
fn no_color_colours_nothing() {
    let p = Palette::new(Tier::Mono);
    assert!(Tok::ALL.iter().all(|t| p.color(*t).is_none()));
}

#[test]
fn sixteen_colours_never_paint_surface_or_selection() {
    let p = Palette::new(Tier::C16);
    assert_eq!(p.color(Tok::Surf), None);
    assert_eq!(p.color(Tok::Sel), None);
}

#[test]
fn a_palette_file_needs_every_token() {
    // Built from the built-in values, with err swapped for accent: the
    // repository may hold no other colours (ADR-003).
    let line = |t: Tok, v: Tok| format!("{} = \"{}\"\n", format!("{t:?}").to_lowercase(), hex(v));
    let full: String = Tok::ALL.iter().map(|t| line(*t, if *t == Tok::Err { Tok::Acc } else { *t })).collect();
    let mut p = Palette::new(Tier::True);
    p.custom = Some(parse_file(&full).unwrap());
    let (r, g, b) = truecolor(Tok::Acc);
    assert_eq!(p.color(Tok::Err), Some(Color::Rgb(r, g, b)));
    assert!(parse_file(&full.replace("err = ", "error = ")).unwrap_err().contains("no err"));
}
