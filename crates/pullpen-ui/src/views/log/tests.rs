use super::*;

fn texts(l: &Log) -> Vec<&str> {
    l.lines.iter().map(|x| x.text.as_str()).collect()
}

/// GitLab's shape, as its runner writes it: a time, a stream prefix, and
/// sections whose headers continue the marker's line.
const GITLAB: &str = "2026-10-27T01:29:57.556268Z 00O \x1b[0KRunning with gitlab-runner 19.3.3\x1b[0;m
2026-10-27T01:29:57.556312Z 00O section_start:1793064597:prepare_executor\r\x1b[0K
2026-10-27T01:29:57.556320Z 00O+\x1b[0K\x1b[36;1mPreparing the \"docker\" executor\x1b[0;m
2026-10-27T01:29:58.000000Z 00O Using docker image golang:1.25.3
2026-10-27T01:29:58.677938Z 00O section_end:1793064598:prepare_executor\r\x1b[0K
2026-10-27T01:29:58.678026Z 00O+section_start:1793064598:step_script[collapsed=true]\r\x1b[0K
2026-10-27T01:29:58.678029Z 00O+\x1b[0K\x1b[36;1mExecuting \"step_script\"\x1b[0;m
2026-10-27T01:29:59.000000Z 00O \x1b[32;1m$ make build\x1b[0;m
2026-10-27T01:30:00.000000Z 01E \x1b[31;1mERROR: Job failed: exit code 2\x1b[0;m
";

#[test]
fn gitlab_sections_become_steps_named_by_their_headers() {
    let l = parse(GITLAB, true);
    assert_eq!(
        texts(&l),
        [
            "Running with gitlab-runner 19.3.3",
            "Preparing the \"docker\" executor",
            "Using docker image golang:1.25.3",
            "Executing \"step_script\"",
            "$ make build",
            "ERROR: Job failed: exit code 2",
        ]
    );
    let secs: Vec<(&str, usize, i64, Option<i64>)> =
        l.sections.iter().map(|x| (x.name.as_str(), x.line, x.started.0, x.finished.map(|f| f.0))).collect();
    assert_eq!(
        secs,
        [
            ("Preparing the \"docker\" executor", 1, 1793064597, Some(1793064598)),
            ("Executing \"step_script\"", 3, 1793064598, None)
        ]
    );
}

#[test]
fn colours_map_onto_the_palette_and_errors_are_marked() {
    let l = parse(GITLAB, true);
    assert_eq!(l.lines[1].spans[0].fg, Tok::Acc, "a section header is cyan");
    assert!(l.lines[1].spans[0].bold);
    assert_eq!(l.lines[4].spans[0].fg, Tok::Ok, "a command is green");
    let errors: Vec<usize> = l.lines.iter().enumerate().filter(|(_, x)| x.error).map(|(i, _)| i).collect();
    assert_eq!(errors, [5]);
    assert!(l.lines.iter().all(|x| x.spans.iter().all(|sp| sp.bg.is_none())), "no background is ever painted");
}

#[test]
fn github_markers_are_read_and_its_times_kept() {
    let raw = "\u{feff}2026-10-27T01:59:07.0223313Z ##[group]Run cargo test
2026-10-27T01:59:07.0224577Z cargo test --workspace
2026-10-27T01:59:07.0225473Z ##[endgroup]
2026-10-27T01:59:09.0000000Z ##[error]Process completed with exit code 101.
2026-10-27T01:59:09.5000000Z ##[warning]Node 16 is deprecated
2026-10-27T01:59:10.0000000Z [command]/usr/bin/git version
";
    let l = parse(raw, false);
    assert_eq!(
        texts(&l),
        [
            "Run cargo test",
            "cargo test --workspace",
            "Process completed with exit code 101.",
            "Node 16 is deprecated",
            "/usr/bin/git version"
        ]
    );
    assert_eq!(l.lines[0].spans[0].fg, Tok::Acc);
    assert_eq!(l.lines[4].spans[0].fg, Tok::Acc, "a command, as the download writes it");
    assert!(l.lines[2].error);
    assert_eq!(l.lines[3].spans[0].fg, Tok::Warn);
    assert_eq!(l.lines[2].at, parse_rfc3339("2026-10-27T01:59:09Z"));
}

#[test]
fn a_redrawn_line_shows_what_is_left_and_other_escapes_go() {
    let l = parse(
        "Downloading 10%\rDownloading 55%\rDownloading 100%\r\n\x1b]0;title\x07a\tb \x1b[38;5;196mred?\x1b[2Kgone\n",
        false,
    );
    assert_eq!(texts(&l), ["Downloading 100%", "a       b red?gone"]);
    assert_eq!(l.lines[1].spans.last().map(|x| x.fg), Some(Tok::Fg), "a 256-colour red is not a palette colour");
}

#[test]
fn a_long_log_keeps_its_tail_and_says_so() {
    let raw: String = (0..MAX_LINES + 5).map(|i| format!("line {i}\n")).collect();
    let l = parse(&raw, false);
    assert!(l.trimmed);
    assert_eq!(l.lines.len(), MAX_LINES);
    assert_eq!(l.lines[0].text, "line 5");
}
