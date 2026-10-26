use super::*;

fn text(l: &Line) -> String {
    l.iter().map(|s| s.text.as_str()).collect()
}

#[test]
fn editing_keeps_the_cursor_in_chars() {
    let mut e = Editor::with("a−b");
    e.left();
    e.insert('x');
    assert_eq!(e.text, "a−xb");
    e.backspace();
    e.backspace();
    assert_eq!(e.text, "ab");
}

#[test]
fn ctrl_w_deletes_the_word_before_the_cursor() {
    let mut e = Editor::with("comment #812 lgtm  ");
    e.delete_word();
    assert_eq!(e.text, "comment #812 ");
    e.delete_line();
    assert_eq!(e.text, "");
}

#[test]
fn the_cursor_is_an_inverted_space_at_the_end() {
    let l = Editor::with("approve").line(':');
    assert_eq!(text(&l), ":approve ");
    let last = l.last().unwrap();
    assert_eq!((last.fg, last.bg), (Tok::Bg, Some(Tok::Fg)));
}

#[test]
fn completion_fills_the_first_item_then_cycles() {
    let mut e = Editor::with("a");
    e.complete(Completion::Command, vec!["approve".into(), "automerge".into()], true);
    assert_eq!(e.text, "approve");
    e.complete(Completion::Command, vec![], true);
    assert_eq!(e.text, "automerge");
    e.complete(Completion::Command, vec![], false);
    assert_eq!(e.text, "approve");
}

#[test]
fn history_walks_only_entries_matching_what_was_typed() {
    let h: Vec<String> =
        ["merge #1", "comment #2 a", "approve", "comment #3 b"].iter().map(|s| s.to_string()).collect();
    let mut e = Editor::with("comment");
    e.walk(&h, true);
    assert_eq!((e.text.as_str(), e.walk_label().unwrap().as_str()), ("comment #3 b", "history 1/2"));
    e.walk(&h, true);
    assert_eq!(e.text, "comment #2 a");
    e.walk(&h, true);
    assert_eq!(e.text, "comment #2 a", "the oldest match is a floor");
    e.walk(&h, false);
    e.walk(&h, false);
    assert_eq!(e.text, "comment", "walking off the newest end restores what was typed");
    assert!(e.walk.is_none());
}
