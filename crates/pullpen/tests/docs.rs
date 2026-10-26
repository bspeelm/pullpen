//! Claims the documents make, checked against the things they describe.

// A test that cannot read its inputs should fail loudly, which is what
// unwrap does; clippy's test allowance does not reach helper functions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn markdown() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let n = e.file_name();
            if n == "target" || n == ".git" || n == "design" {
                continue;
            }
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "md") {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    walk(&root(), &mut v);
    v
}

fn makefile_var(name: &str) -> u64 {
    let mk = read("Makefile");
    let line = mk
        .lines()
        .find(|l| l.split_whitespace().next() == Some(name))
        .unwrap_or_else(|| panic!("{name} not in the Makefile"));
    line.rsplit(' ').next().unwrap().trim().parse().unwrap()
}

fn thousands(n: u64) -> String {
    if n >= 1000 {
        format!("{},{:03}", n / 1000, n % 1000)
    } else {
        n.to_string()
    }
}

#[test]
fn the_documented_budgets_match_the_makefile() {
    let adr = read("docs/decisions.md");
    let want = [
        format!("{} MB", makefile_var("MAX_BINARY_BYTES") / (1024 * 1024)),
        format!("{} lines", thousands(makefile_var("MAX_CODE_LINES"))),
        format!("{}% of the code", makefile_var("MAX_COMMENT_RATIO")),
        format!("{}% of the code", makefile_var("MAX_DOC_RATIO")),
        format!(
            "{} lines of code",
            thousands(makefile_var("DOC_RATIO_FROM"))
        ),
    ];
    for w in want {
        assert!(
            adr.contains(&w),
            "ADR-004 does not say \"{w}\", which is what the Makefile enforces"
        );
    }
}

#[test]
fn every_relative_doc_link_resolves() {
    let mut broken = Vec::new();
    for md in markdown() {
        let text = fs::read_to_string(&md).unwrap();
        let mut rest = text.as_str();
        while let Some(i) = rest.find("](") {
            rest = &rest[i + 2..];
            let Some(end) = rest.find(')') else { break };
            let target = &rest[..end];
            let file = target.split('#').next().unwrap();
            if file.is_empty() || target.contains("://") || target.starts_with("mailto:") {
                continue;
            }
            if !md.parent().unwrap().join(file).exists() {
                broken.push(format!(
                    "{}: {target}",
                    md.strip_prefix(root()).unwrap().display()
                ));
            }
        }
    }
    assert!(
        broken.is_empty(),
        "links to files that do not exist:\n{}",
        broken.join("\n")
    );
}

#[test]
fn the_config_reference_is_valid_toml() {
    let plan = read("docs/PLAN.md");
    let section = &plan[plan.find("## 9. Config reference").expect("PLAN.md §9")..];
    let start = section.find("```toml\n").expect("a toml block in §9") + 8;
    let end = start + section[start..].find("```").unwrap();
    let doc: toml::Table = toml::from_str(&section[start..end])
        .unwrap_or_else(|e| panic!("PLAN.md §9 is not valid TOML: {e}"));
    assert_eq!(
        doc["section"].as_array().map(Vec::len),
        Some(5),
        "the five default sections"
    );
}

#[test]
fn every_flag_the_plan_names_for_m0_exists() {
    let help = Command::new(env!("CARGO_BIN_EXE_pullpen"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    for flag in [
        "--config",
        "--fixtures",
        "--glyphs",
        "--colors",
        "--section",
        "--version",
    ] {
        assert!(
            help.contains(flag),
            "PLAN.md M0 names {flag}, but `pullpen --help` does not offer it"
        );
    }
}
