use super::*;

/// The collapse table in design §11, boundary by boundary.
#[test]
fn columns_collapse_in_the_designed_order() {
    let at = |w| {
        let c = cols(w);
        (c.size, c.author, c.repo, c.age, c.rev, c.forge)
    };
    assert_eq!(at(89), (12, 12, 17, 4, 5, 3), "≥ 89: everything");
    assert_eq!(at(88), (0, 12, 17, 4, 5, 3), "77–88: size dropped");
    assert_eq!(at(77), (0, 12, 17, 4, 5, 3));
    assert_eq!(
        at(76),
        (0, 2, 17, 4, 5, 3),
        "67–76: author is the mark only"
    );
    assert_eq!(at(67), (0, 2, 17, 4, 5, 3));
    assert_eq!(at(66), (0, 2, 11, 4, 5, 3), "61–66: repo shortens");
    assert_eq!(at(60), (0, 2, 11, 0, 5, 3), "57–60: age dropped");
    assert_eq!(at(56), (0, 2, 11, 0, 0, 3), "52–56: reviews dropped");
    assert_eq!(at(51), (0, 2, 11, 0, 0, 0), "49–51: forge dropped");
    assert_eq!(at(48), (0, 2, 0, 0, 0, 0), "38–48: repo dropped");
    assert_eq!(cols(38).title, 24);
    assert_eq!(cols(30).title, 16, "< 38: title truncates below 24");
}

#[test]
fn the_title_takes_all_spare_width() {
    let c = cols(119);
    assert_eq!(
        12 + c.forge + c.repo + c.author + c.rev + c.size + c.age + c.title,
        119
    );
}

#[test]
fn title_match_is_case_insensitive_and_underlined() {
    let segs = title_segs("Migrate invoices to Ledger v2", Some("ledger"), Tok::Fg);
    assert_eq!(segs[1].text, "Ledger");
    assert!(segs[1].underline);
    assert_eq!(segs[1].fg, Tok::Acc);
}
