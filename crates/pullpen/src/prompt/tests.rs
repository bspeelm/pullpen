use std::io::Cursor;

use super::*;

fn run<T>(script: &str, f: impl FnOnce(&mut Io) -> io::Result<T>) -> (io::Result<T>, String) {
    let mut input = Cursor::new(script.as_bytes().to_vec());
    let mut out = Vec::new();
    let mut secret = || Ok("s3cret".to_string());
    let r = f(&mut Io { input: &mut input, out: &mut out, secret: &mut secret });
    (r, String::from_utf8(out).unwrap())
}

#[test]
fn an_empty_answer_takes_the_default() {
    let (r, out) = run("\n", |io| io.line("host", "github.com"));
    assert_eq!(r.unwrap(), "github.com");
    assert!(out.contains("host [github.com]: "));
}

#[test]
fn yes_no_asks_again_until_it_understands() {
    let (r, out) = run("maybe\nn\n", |io| io.yes("Delete the branch", true));
    assert!(!r.unwrap());
    assert!(out.contains("y or n"));
}

#[test]
fn pick_takes_a_number_or_a_name() {
    assert_eq!(run("2\n", |io| io.pick("method", &["repo", "squash"], 0)).0.unwrap(), 1);
    assert_eq!(run("Squash\n", |io| io.pick("method", &["repo", "squash"], 0)).0.unwrap(), 1);
    assert_eq!(run("9\n\n", |io| io.pick("method", &["repo", "squash"], 0)).0.unwrap(), 0);
}

#[test]
fn hidden_answers_are_not_echoed() {
    let (r, out) = run("", |io| io.hidden("token"));
    assert_eq!(r.unwrap(), "s3cret");
    assert!(!out.contains("s3cret"));
}

#[test]
fn running_out_of_answers_is_an_error_not_a_loop() {
    assert!(run("", |io| io.yes("ok", true)).0.is_err());
}
