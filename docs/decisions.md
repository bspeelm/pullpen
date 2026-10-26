# Decisions

Why pullpen is the way it is. A decision recorded here is settled; proposing
it again means answering the record by number, not starting the argument over.
A record that is overturned stays, with a later record saying what replaced it.

---

## ADR-001 — The core is Rust

pullpen draws every screen cell by cell to match a design that was itself
specified cell by cell (`design/tui.js`). That makes control of the cell buffer
the requirement. Startup has to be instant, because the tool is opened and
closed many times a day, and it ships as one binary with nothing to install
first.

Rust with `ratatui` and `crossterm` gives that buffer directly. `tokio` keeps
forge calls off the UI thread without hand-built threading.

This departs from bothy's ADR-001, *the core is Go*, and the departure is
deliberate. bothy is glue: it writes config files and launches other programs,
and Go's cross-compilation and small dependency story fit that. pullpen is the
program the user is looking at. Its hard parts are rendering and concurrent
network state, where Rust's libraries are stronger. Everything else bothy does
carries over unchanged: the records, the budgets, the doc tests and the
release discipline.

**What it costs.** Cross-compiling for macOS from Linux takes a toolchain this
project's own machine does not have, so the macOS claim rests on a CI job
(ADR-004), not on `make check`. Compile times are longer than Go's.

## ADR-002 — The design artifact is the visual spec, and the goldens are its proof

`design/` is the design exactly as it was delivered and reviewed. `tui.js`
renders every screen from fixture data without a browser, which turns
"matches the design" into something a test can check rather than an opinion.

`scripts/gen-golden.mjs` runs it and writes `tests/golden/`. The Rust renderer
draws the same fixtures, and the tests compare the two byte for byte. When the
look needs to change, `tui.js` changes first, the goldens are regenerated, and
the Rust code follows. Any other order leaves the design describing a product
that does not exist.

Node is needed only to regenerate the goldens. The tests read the committed
output, so Node is not a build dependency. CI uses it for one job, which
regenerates the goldens and fails if they differ from what is committed.

Where this plan and the design disagree about how something looks, the design
wins. Where they disagree about how something works, the plan wins.

## ADR-003 — One palette ships; every other one is a file you supply

pullpen carries exactly one palette: the design's ten tokens in their
truecolor, 256-colour and 16-colour tiers. Any other palette, whether Dracula
or one you have licensed, arrives as a TOML file of the same ten tokens that
`ui.palette` points at. `ui.accent` overrides the accent alone.

This is bothy's ADR-006, kept in the same shape and for the same reason. A
palette someone licensed stays on their machine, and the way in for it is the
same as for every other theme. A test enforces it by inverting the check: it
scans every shipped file and fails on any colour value that is not one of the
built-in palette's. It names nothing it forbids, so it catches any stray
palette, not just the one somebody thought of.

The design files are exempt from the scan because they are the source of the
palette. A colour that appears there and not in the built-in palette is a sign
the two have drifted, and the golden tests catch that sooner.

## ADR-004 — The budgets, and why they are failing checks

`make check` fails when any of these is crossed:

- the stripped release binary is larger than 10 MB;
- the code is longer than 8,000 lines;
- comments are more than 30% of the code;
- prose is more than 75% of the code, once the code reaches 2,000 lines.

They are checks, not aims, because a limit that only warns gets exceeded on
the first busy day. The numbers live in the `Makefile`, and a test fails if
any document quotes them differently.

Code and comments are counted separately, as in bothy's ADR-010 and ADR-021.
A single combined figure punishes a comment explaining *why* exactly as hard
as it punishes sprawl. Code means the non-test `.rs` files under
`crates/*/src`. Unit tests live in a sibling `tests.rs`, never inline, so the
counter can tell code from tests by filename alone.

Prose means every markdown file outside `design/` and `docs/history/`. It is
capped because writing about a project grows to fill whatever room it is
given. The cap starts at 2,000 lines of code because the plan alone is longer
than 75% of an empty repository. Before that point the ratio measures how new
the project is, not whether the prose has run away.

The code figure is an estimate made before the forge clients exist. When work
presses against it, the answer is a new record that measures the overrun and
says what the extra lines buy. Quietly editing the `Makefile` is not an answer.

## ADR-005 — The dependency ceiling is a list

The crates pullpen may depend on are the ones listed in `PLAN.md` §1. Adding
one takes a record here first, saying what it replaces and what it weighs.

bothy's version of this rule is "ask before adding a dependency". Here the
same rule is checkable: the list is in the plan, `Cargo.toml` can be compared
against it, and a new entry without a record stands out in review.
Transitive dependencies are not listed. They are the cost of the direct ones,
and the binary budget (ADR-004) is what keeps that cost honest.

## ADR-006 — pullpen never stores a token

Credentials come from the environment (`PULLPEN_GITHUB_TOKEN`, then
`GITHUB_TOKEN`), from `gh auth token` or `glab`, or from a `token_cmd` the
user configures, in that order. They live in memory for the life of the
process and are never written to the cache, the log, or the message history.

Anyone already using `gh` or `glab` has nothing to set up. More importantly,
pullpen holds no secret worth stealing. A tool that kept its own token store
would need to protect it, rotate it and explain it, and none of that would
make it better at its job.

The log must redact the `Authorization` header. The forge milestone (M3) is
not done until a test asserts that a known token never appears in any file
pullpen writes.
