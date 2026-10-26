//! `pullpen config check`: what the config asks for, whether each part of
//! it works, and for every part that does not, how to fix it.

use std::path::Path;

use pullpen_core::Forge;
use pullpen_core::query::{Query, Term};
use pullpen_forge::auth;

use super::{Config, Kind, expand};
use crate::credentials::Credentials;

pub struct Finding {
    pub ok: bool,
    pub what: String,
    /// Required when `ok` is false; a test holds every failure to it.
    pub fix: Option<String>,
}

fn ok(what: String) -> Finding {
    Finding { ok: true, what, fix: None }
}

fn bad(what: String, fix: &str) -> Finding {
    Finding { ok: false, what, fix: Some(fix.into()) }
}

pub fn check(
    path: &Path,
    cfg: &Config,
    load_err: Option<&str>,
    creds: Result<&Credentials, &str>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Vec<Finding> {
    let mut out = vec![match load_err {
        Some(e) => bad(e.to_string(), "correct the line it names, or move the file aside to use the defaults"),
        None if path.exists() => ok(format!("{} reads", path.display())),
        None => ok(format!("no {}; using the defaults", path.display())),
    }];
    let empty = Credentials::default();
    let creds = match creds {
        Ok(c) => c,
        Err(e) => {
            out.push(bad(e.to_string(), "run the chmod it names, or `pullpen setup` to store the token again"));
            &empty
        }
    };
    for f in cfg.forges() {
        let kind = match f.kind {
            Kind::Github => Forge::GitHub,
            Kind::Gitlab => Forge::GitLab,
        };
        out.push(match auth::token(kind, f.host(), creds.get(f.host()), f.token_cmd.as_deref(), env) {
            Ok(_) => ok(format!("{}: signed in", f.host())),
            Err(_) => bad(
                format!("{}: no token", f.host()),
                match kind {
                    Forge::GitHub => "run `pullpen setup`, or `gh auth login`, or set PULLPEN_GITHUB_TOKEN",
                    Forge::GitLab => "run `pullpen setup`, or `glab auth login`, or set PULLPEN_GITLAB_TOKEN",
                },
            ),
        });
    }
    for s in cfg.sections() {
        let odd: Vec<String> = Query::parse(&s.query)
            .terms
            .into_iter()
            .filter_map(|t| if let Term::Text(t) = t { t.contains(':').then_some(t) } else { None })
            .collect();
        out.push(if odd.is_empty() {
            ok(format!("section {}: {}", s.name, s.query))
        } else {
            bad(
                format!("section {}: {} is not a query term, so it matches titles", s.name, odd.join(", ")),
                "check the term against docs/PLAN.md §6.1",
            )
        });
    }
    let (_, keys) = pullpen_ui::keymap::remaps(&cfg.keys);
    out.extend(
        keys.into_iter().map(|k| bad(k, "use an action name from docs/PLAN.md §9 and a key like M, ^x or Space")),
    );
    if let Some(p) = &cfg.ui.palette {
        let read = std::fs::read_to_string(expand(p))
            .map_err(|e| e.to_string())
            .and_then(|t| pullpen_ui::palette::parse_file(&t));
        out.push(match read {
            Ok(_) => ok(format!("palette {p}")),
            Err(e) => bad(format!("palette {p}: {e}"), "give all ten tokens as `name = \"#rrggbb\"` (ADR-003)"),
        });
    }
    out
}

/// Prints the findings; the exit status says whether any failed.
pub fn report(findings: &[Finding]) -> bool {
    for f in findings {
        println!("{} {}", if f.ok { "✓" } else { "✗" }, f.what);
        if let Some(fix) = &f.fix {
            println!("    fix: {fix}");
        }
    }
    findings.iter().all(|f| f.ok)
}
