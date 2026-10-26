//! The fixture data from `design/tui.js` (`P`, `H`, `ALL`, the sections) as
//! Rust, for `pullpen --fixtures` and for the golden tests (PLAN.md §7).
//!
//! Rows are transcribed in tui.js's compact form and expanded into full
//! `Pr`s here, so a reader can check them against the design line by line.

use pullpen_core::time::{Timestamp, parse_age};
use pullpen_core::*;

/// The instant every fixture age is measured from. Fixed, so screens render
/// the same on every run.
pub const NOW: Timestamp = Timestamp(1_790_000_000);

/// tui.js `AG`: the logins the design marks as agents.
pub const AGENTS: [&str; 5] = ["claude", "codex", "renovate", "dependabot", "devin"];

#[derive(Clone, Copy)]
enum Ci {
    Ok,
    Fail,
    Run,
    None,
}

struct Raw {
    f: Forge,
    r: &'static str,
    n: u64,
    t: &'static str,
    a: &'static str,
    ci: Ci,
    rv: (u32, u32),
    armed: bool,
    draft: bool,
    conflict: bool,
    ad: u32,
    de: u32,
    age: &'static str,
}

// One positional argument per tui.js field, so each row below reads like
// the design's own line.
#[allow(clippy::too_many_arguments)]
const fn raw(
    f: Forge,
    r: &'static str,
    n: u64,
    t: &'static str,
    a: &'static str,
    ci: Ci,
    rv: (u32, u32),
    ad: u32,
    de: u32,
    age: &'static str,
) -> Raw {
    Raw {
        f,
        r,
        n,
        t,
        a,
        ci,
        rv,
        armed: false,
        draft: false,
        conflict: false,
        ad,
        de,
        age,
    }
}

const fn armed(mut x: Raw) -> Raw {
    x.armed = true;
    x
}
const fn draft(mut x: Raw) -> Raw {
    x.draft = true;
    x
}
const fn conflict(mut x: Raw) -> Raw {
    x.conflict = true;
    x
}

use Ci::{Fail, Ok, Run};
use Forge::{GitHub as GH, GitLab as GL};

#[rustfmt::skip]
const P: [Raw; 23] = [
    armed(raw(GH, "acme/api", 4821, "Migrate invoices to ledger v2", "claude", Ok, (1, 2), 412, 88, "3h")),
    raw(GL, "platform/runner", 812, "Bump go toolchain to 1.25.3", "renovate", Ok, (1, 1), 6, 6, "41m"),
    armed(raw(GH, "acme/web", 2290, "Fix hydration mismatch on settings page when locale is RTL", "codex", Fail, (0, 1), 38, 12, "5h")),
    raw(GH, "acme/infra", 377, "Rotate staging TLS certs via cert-manager", "claude", Run, (0, 1), 54, 20, "12m"),
    raw(GL, "platform/charts", 145, "Add PodDisruptionBudget to ingest workers", "claude", Ok, (1, 1), 31, 0, "1h"),
    armed(conflict(raw(GH, "acme/api", 4818, "Add idempotency keys to POST /payments", "codex", Ok, (2, 2), 220, 41, "1d"))),
    raw(GH, "tools/rq", 96, "Cache forge capabilities per host", "claude", Ok, (1, 1), 73, 15, "2h"),
    armed(raw(GH, "acme/web", 2288, "chore(deps): update dependency vite to v7.1.4", "dependabot", Ok, (0, 1), 3, 3, "6h")),
    armed(raw(GL, "platform/runner", 809, "Retry artifact upload on 503 with jittered backoff", "codex", Run, (1, 1), 64, 9, "30m")),
    draft(raw(GH, "acme/billing", 1203, "Proration preview endpoint", "claude", Ci::None, (0, 2), 388, 2, "2d")),
    raw(GH, "acme/api", 4815, "Remove deprecated v1 webhooks handler", "claude", Ok, (2, 2), 12, 540, "2d"),
    raw(GL, "platform/charts", 143, "Pin redis chart to 19.6.x", "renovate", Fail, (0, 1), 2, 2, "3d"),
    raw(GH, "acme/infra", 375, "Terraform: split vpc module by region", "codex", Ok, (0, 1), 610, 402, "3d"),
    armed(raw(GH, "acme/billing", 1199, "Ledger: emit backfill progress metrics", "claude", Ok, (1, 1), 47, 3, "4h")),
    raw(GL, "platform/runner", 806, "Drop support for docker-machine executor", "devin", Ok, (0, 2), 12, 1340, "4d"),
    raw(GH, "acme/web", 2284, "Expose ledger balances in admin account view", "codex", Run, (0, 1), 156, 22, "7h"),
    armed(raw(GH, "acme/api", 4809, "Bump pgx to v5.7.2", "renovate", Ok, (1, 1), 4, 4, "5d")),
    raw(GL, "platform/docs", 58, "Document runner autoscaling flags", "claude", Ok, (0, 1), 88, 10, "5d"),
    raw(GH, "tools/rq", 94, "Map GitLab approval rules to required reviews", "claude", Ok, (1, 1), 132, 40, "6d"),
    raw(GH, "acme/infra", 371, "Enable VPC flow logs to S3 with 30-day retention", "devin", Fail, (0, 1), 41, 0, "6d"),
    raw(GH, "acme/web", 2279, "chore(deps): update dependency @types/node to v22.18", "dependabot", Ok, (0, 1), 2, 2, "1w"),
    raw(GL, "platform/charts", 139, "Bump ingress-nginx to 4.13.1", "renovate", Ok, (1, 1), 2, 2, "1w"),
    raw(GH, "acme/api", 4796, "Trace context propagation for outbound HTTP", "codex", Ok, (0, 1), 95, 18, "2w"),
];

#[rustfmt::skip]
const H: [Raw; 12] = [
    raw(GH, "acme/api", 4823, "Rate limit per API key instead of per IP", "mara", Run, (0, 1), 140, 31, "20m"),
    raw(GH, "acme/web", 2291, "New billing settings layout", "jkoh", Ok, (1, 1), 820, 615, "1h"),
    raw(GL, "platform/runner", 814, "Ledger exporter for runner minutes", "ash", Fail, (0, 1), 203, 14, "2h"),
    raw(GH, "acme/billing", 1205, "Invoice PDF: handle multi-currency totals", "priya", Ok, (0, 2), 96, 30, "4h"),
    raw(GL, "platform/docs", 60, "Fix broken links in runner install guide", "lena", Ok, (1, 1), 9, 9, "5h"),
    raw(GH, "acme/infra", 378, "Grafana: alert on ledger write latency p99", "ash", Ok, (0, 1), 58, 0, "8h"),
    raw(GH, "acme/api", 4811, "Split payments service config by environment", "mara", Ok, (2, 2), 77, 65, "1d"),
    raw(GL, "platform/charts", 146, "Allow custom annotations on runner pods", "tomasz", Run, (0, 1), 22, 3, "1d"),
    draft(raw(GH, "acme/web", 2286, "Keyboard shortcuts for the invoice table", "jkoh", Ok, (0, 1), 140, 12, "2d")),
    raw(GH, "tools/rq", 97, "Command-line history persists across sessions", "ash", Ok, (0, 0), 66, 8, "2d"),
    raw(GH, "acme/api", 4802, "Upgrade to Go 1.25", "mara", Fail, (0, 1), 31, 29, "3d"),
    raw(GL, "platform/runner", 801, "Shell executor: honor umask from config", "lena", Ok, (1, 1), 15, 2, "1w"),
];

fn ago(age: &str) -> Timestamp {
    Timestamp(NOW.0 - parse_age(age).unwrap_or(0))
}

fn expand(x: &Raw) -> Pr {
    let host = match x.f {
        Forge::GitHub => "github.com",
        Forge::GitLab => "gitlab.com",
    };
    let (total, passed, running, failed) = match (x.n, x.ci) {
        (4821, _) => (12, 12, 0, vec![]),
        (377, _) => (9, 4, 5, vec![]),
        (809, _) => (9, 6, 3, vec![]),
        (2290, Fail) => (
            9,
            8,
            0,
            vec![CheckRun {
                name: "e2e/chromium".into(),
                detail: "2 retries".into(),
            }],
        ),
        (_, Ok) => (9, 9, 0, vec![]),
        (_, Run) => (9, 4, 5, vec![]),
        (_, Fail) => (
            9,
            8,
            0,
            vec![CheckRun {
                name: "test".into(),
                detail: String::new(),
            }],
        ),
        (_, Ci::None) => (0, 0, 0, vec![]),
    };
    let pending = match x.n {
        4821 => vec!["mara".to_string()],
        _ => vec![],
    };
    let head = match x.n {
        4821 => "claude/ledger-v2".to_string(),
        n => format!("{}/{n}", x.a),
    };
    let sep = if x.f == Forge::GitHub {
        "pull"
    } else {
        "-/merge_requests"
    };
    Pr {
        id: PrId {
            forge: x.f,
            host: host.into(),
            repo: x.r.into(),
            number: x.n,
        },
        title: x.t.into(),
        author: Author {
            login: x.a.into(),
            is_agent: AGENTS.contains(&x.a),
        },
        head,
        base: "main".into(),
        additions: x.ad,
        deletions: x.de,
        files: if x.n == 4821 { 14 } else { 3 },
        draft: x.draft,
        checks: Checks {
            required_total: total,
            required_passed: passed,
            running,
            failed,
        },
        reviews: Reviews {
            have: x.rv.0,
            need: x.rv.1,
            changes_requested: false,
            pending,
        },
        mergeable: if x.conflict {
            Mergeable::Conflict
        } else {
            Mergeable::Clean
        },
        auto_merge: x.armed.then(|| AutoMerge {
            method: MergeMethod::Squash,
            armed_by: "ash".into(),
            armed_at: ago("2h"),
        }),
        merge_train: None,
        state: PrState::Open,
        updated_at: ago(x.age),
        created_at: ago(x.age),
        url: format!("https://{host}/{}/{sep}/{}", x.r, x.n),
        repo_default_method: MergeMethod::Squash,
        repo_allowed_methods: vec![MergeMethod::Squash, MergeMethod::Merge, MergeMethod::Rebase],
    }
}

/// tui.js `P`: the agent-opened PRs.
pub fn agent_prs() -> Vec<Pr> {
    P.iter().map(expand).collect()
}

/// tui.js `H`: PRs opened by people.
pub fn human_prs() -> Vec<Pr> {
    H.iter().map(expand).collect()
}

/// tui.js `ALL`: one human PR, then two agent ones, until both run out.
pub fn all_prs() -> Vec<Pr> {
    let (p, h) = (agent_prs(), human_prs());
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < p.len() || j < h.len() {
        if j < h.len() {
            out.push(h[j].clone());
            j += 1;
        }
        for _ in 0..2 {
            if i < p.len() {
                out.push(p[i].clone());
                i += 1;
            }
        }
    }
    out
}

pub fn by_number(n: u64) -> Option<Pr> {
    all_prs().into_iter().find(|p| p.id.number == n)
}

/// A section with its rows, and the total the forge reported, which can
/// exceed the rows loaded (tui.js shows `All 96` over 35 rows).
pub struct FixtureSection {
    pub section: Section,
    pub prs: Vec<Pr>,
    pub total: usize,
}

fn section(
    name: &str,
    short: &str,
    query: &str,
    view: View,
    counts: bool,
    prs: Vec<Pr>,
    total: usize,
) -> FixtureSection {
    FixtureSection {
        section: Section {
            name: name.into(),
            short: short.into(),
            query: query.into(),
            sort: Sort::Updated,
            view,
            counts,
        },
        prs,
        total,
    }
}

/// tui.js `SECS`, with rows. Mine and Review are drawn from `H`; the design
/// gives only their counts.
pub fn sections() -> Vec<FixtureSection> {
    let pick = |ns: &[u64]| ns.iter().filter_map(|n| by_number(*n)).collect::<Vec<_>>();
    let armed: Vec<Pr> = agent_prs().into_iter().filter(Pr::armed).collect();
    vec![
        section(
            "Mine",
            "Mine",
            "is:open author:@me",
            View::List,
            true,
            pick(&[4823, 2291, 1205, 60, 378, 4811]),
            6,
        ),
        section(
            "Needs my review",
            "Review",
            "is:open review-requested:@me",
            View::List,
            true,
            pick(&[2291, 1205, 378, 146]),
            4,
        ),
        section(
            "Agent-opened",
            "Agent",
            "is:open is:agent",
            View::List,
            true,
            agent_prs(),
            23,
        ),
        section(
            "Auto-merge armed",
            "Armed",
            "is:armed",
            View::Armed,
            true,
            armed.clone(),
            armed.len(),
        ),
        section("All", "All", "is:open", View::List, false, all_prs(), 96),
    ]
}

/// The preview and detail data the design shows for #4821. Other PRs get an
/// empty detail.
pub fn detail(n: u64) -> PrDetail {
    if n != 4821 {
        return PrDetail::default();
    }
    let who = |w: &str| (w.to_string(), AGENTS.contains(&w));
    let act = |w: &str, kind: ActivityKind, age: &str, body: &str| {
        let (who, is_agent) = who(w);
        Activity {
            who,
            is_agent,
            kind,
            at: ago(age),
            body: body.into(),
        }
    };
    let ck = |name: &str, d: &str| CheckLine {
        name: name.into(),
        state: CheckState::Passed,
        required: true,
        duration: d.into(),
    };
    let mut checks = vec![
        ck("build", "1m 52s"),
        ck("test / unit", "3m 10s"),
        ck("test / integration", "7m 41s"),
        ck("lint", "38s"),
        ck("migrations / dry-run", "2m 05s"),
    ];
    for i in 0..7 {
        checks.push(ck(&format!("check {}", i + 6), "1m 00s"));
    }
    PrDetail {
        description: "Moves invoice writes onto the ledger v2 double-entry tables. The backfill runs as an online migration in batches of 5,000 with a checkpoint table, so it resumes cleanly after a deploy. Reads switch behind the ledger_v2_reads flag, off by default.\n\n- add migrate_invoices job and checkpoint table\n- dual-write legacy + ledger until the flag flips\n- remove InvoiceTotalsCache, unused since #4790".into(),
        checks,
        activity: vec![
            act("jkoh", ActivityKind::Approved, "2h", ""),
            act("ash", ActivityKind::ArmedAutoMerge, "2h", ""),
            act("claude", ActivityKind::Pushed { commits: 2 }, "3h", ""),
            act("mara", ActivityKind::ReviewRequested { by: None }, "3h", ""),
        ],
        conversation: vec![
            act("jkoh", ActivityKind::Approved, "2h", "Batching looks right. The checkpoint table needs an index on invoice_id."),
            act("claude", ActivityKind::Pushed { commits: 1 }, "1h", "add index on checkpoint(invoice_id)"),
            act("mara", ActivityKind::ReviewRequested { by: Some("claude".into()) }, "3h", ""),
        ],
    }
}

#[cfg(test)]
mod tests;

/// A golden screen from `tests/golden/`: `(plain, styled)`.
pub fn golden(name: &str) -> (String, String) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let read = |f: String| {
        std::fs::read_to_string(dir.join(&f))
            .unwrap_or_else(|e| panic!("{f}: {e}; run `make golden`"))
    };
    (
        read(format!("{name}.txt")),
        read(format!("{name}.ansi.txt")),
    )
}
