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
`crates/*/src`, as formatted by `rustfmt.toml`: a narrower format would
report more lines for the same code, so the format changes only by a record. Unit tests live in a sibling `tests.rs`, never inline, so the
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

**Amended by ADR-012:** pullpen now stores a token it is given in setup.

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

## ADR-007 — The batch strip marks rows the design's own rule can act on

The design's batch example marked #812, #145, #96, #1203, #4815 and #375,
and its result showed #812 merging at once ("already green") while #145, #96
and #4815 were armed and left waiting. Its note says the rule: rows already
green merge immediately. But #145, #96 and #4815 are green in the same
fixtures; the rows show ✓. No one rule produces that screen.

The rule is the useful half. Arming a PR that could merge now only makes the
user wait for a merge nobody is blocking, and GitHub refuses to enable
auto-merge on a PR whose state is already clean. So the example changed and
the rule did not: the marks in `design/tui.js` are now #812, #377, #143,
#1203 and #375, of which only #812 is green, and the counts and message follow
(5 marked, 4 armed, 11 armed in total). The goldens were regenerated from the
edited design, as ADR-002 says changes to the look must be.

## ADR-008 — GitLab is read through GraphQL and written through REST

`PLAN.md` chose REST v4 for GitLab, with GraphQL "optional later". Building
the list showed what that costs: REST's merge request list has no head
pipeline, no approvals and no line counts, so each row needs three more
requests, which is 150 calls for a page of 50.

GitLab's GraphQL returns all of it in one query per page: `diffStatsSummary`,
`headPipeline`, `approvalsRequired`, `approvalsLeft`, `approvedBy`,
`detailedMergeStatus`, `autoMergeEnabled`, `mergeTrainIndex`. Every field
pullpen reads was checked against the gitlab.com schema by introspection
before the client was written, rather than taken from memory.

Writes stay REST (`/merge`, `/approve`, `/notes`,
`/cancel_merge_when_pipeline_succeeds`), because the REST endpoints are the
documented, long-stable interface for them and GraphQL's merge mutations lag
behind.

GitHub was already split the same way, for the same reason: GraphQL for
reading, REST for the merge, review, comment and close endpoints.

## ADR-009 — The armed example's #4809 shows that it is waiting

In the design's armed view, #4809 sits under *waiting* with the reason
"required check sbom queued", but its state glyph was ✓, ready. A PR
waiting on a required check is not ready, and the glyph is the one cell the
design asks the user to trust at a glance. So the example changed, as
ADR-007's did: `design/tui.js` gives #4809 a running check there, its row
shows ○, and the goldens were regenerated.

## ADR-010 — PR numbers are not OSC 8 links yet

The design asks for PR numbers as OSC 8 hyperlinks, so cmd-click opens them.
ratatui 0.30's cells carry a symbol and a style and nothing else; there is
no hyperlink attribute. The workaround in circulation writes the escape
sequence into a cell's symbol, where ratatui measures it as text when
deciding which cells to redraw, and the layout this project tests byte for
byte (ADR-002) would rest on that measurement going right.

`o` opens the selected PR in the browser, so nothing is unreachable without
links. This is deferred, not refused: it returns when ratatui gains a link
attribute, or with a test that proves the workaround leaves every golden
screen intact.

## ADR-011 — Releases are a workflow in this repository, not cargo-dist

`PLAN.md` named `cargo-dist`. It is not installed where this project is
built, so the workflow it would generate could not be read and run here
before being trusted, which is the opposite of what bothy's release
records ask for.

`.github/workflows/release.yml` is about a hundred lines. It runs `make
check`, refuses a tag that disagrees with `Cargo.toml`, builds four targets,
packages them under fixed names, writes `checksums.txt`, attests every
artifact including the checksums (bothy's ADR-030), and publishes. Each step
can be read in a minute. cargo-dist would add installers and a Homebrew
tap update in exchange for generated code, and either can be taken up later
by a record that replaces this one.

## ADR-012 — Setup stores the tokens it is given, owner-only, beside the config

ADR-006 had pullpen borrow tokens and never keep one. That serves someone who
already uses `gh` or `glab`, and nobody else: anyone else had to export an
environment variable in every shell before pullpen worked at all. The
maintainer's requirement is that someone can configure pullpen by answering
its questions, so `pullpen setup` asks for a token when it finds no login to
borrow, and keeps it.

**Where.** `~/.config/pullpen/credentials.toml`, mode 0600, written to a new
owner-only file and renamed into place so the token is never on disk with
wider permissions, even briefly. It is separate from `config.toml` so that
file can be shared or committed. This is how `gh` and `glab` store theirs.
An OS keyring would encrypt at rest, but it adds a dependency (ADR-005) and
fails on headless machines with no keyring running.

**What still holds from ADR-006.** A login to borrow is offered first, and
when chosen nothing is stored. The environment still overrides the file. A
token never reaches the log, the cache, the history or the screen: setup
reads it with echo off, and the test that holds ADR-006 still runs. A
credentials file others can read is refused with the `chmod` that fixes it,
not used.

## ADR-013 — The code cap rises to 9,000, for setup and settings

`pullpen setup` and `pullpen settings` took the code from 7,388 lines to
8,049, over ADR-004's 8,000. They are the maintainer's requirement, since
without them pullpen could be configured only by editing TOML, and they were
written as plain prompts rather than as a TUI screen to stay small.

The cap rises once, to 9,000 lines, which is bothy's figure. That leaves about 950
lines, against roughly 500 that the publishing work in `docs/launch.md` is
likely to need, so the next raise has room to be argued properly rather
than squeezed.
