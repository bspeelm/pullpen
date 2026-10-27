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

## ADR-014 — Files are `n` and `b`, the list is `F`, and the detail view says so

The design moved between files with `]f` and `[f`, vim's spelling. Reviewing
a PR of 133 files showed what that costs: the maintainer had to ask what
`]f` meant, and once known it still stepped through files one at a time with
no way to see them all.

So the detail view takes `n` for the next file and `b` for the one before,
`F` opens a list of every changed file to filter and jump from, and its
bottom row, empty in the design, names those keys. `]f` and `[f` still work.
The design changed first, as ADR-002 requires: `design/tui.js` gained the
bottom row and the help overlay's entries, and the goldens were regenerated
from it.

## ADR-015 — `u` updates a branch that is behind its base

A repository that requires branches to be up to date leaves a PR waiting
with nothing for pullpen to do: arming it waits for an update nobody makes,
and the forge's own button was the only way on. So `u` and `:update` ask
the forge to bring the branch up to date, by its own means: GitHub merges
the base in, GitLab rebases, and the message says which happened.

It does not confirm for one PR, since it adds nothing that was not already
on the base, and asks once for several. `M` on a PR whose only wait is the
base refuses and names `u` instead of offering to arm. The design changed
first, as ADR-002 requires: the help overlay's Act column gained `u`, and
the goldens were regenerated from it.

## ADR-016 — Windows is supported

Windows was out of scope for v1 (PLAN.md §10). The maintainer requires it,
and the cost turned out small: crossterm drives the Windows console,
`.gitattributes` already keeps the goldens LF on a Windows checkout, and the
key loops already drop the key-release events Windows sends.

**Where files go.** Windows' own folders, since nothing on Windows looks in
`~/.config`: settings and the stored token in `%APPDATA%\pullpen`, the cache
and log in `%LOCALAPPDATA%\pullpen\cache` and `\state`. `XDG_*` variables
still win when set, for shells that set them on purpose.

**The stored token.** On Linux and macOS it is created 0600 and refused if
others can read it (ADR-012). On Windows it relies on `%APPDATA%`'s ACL,
which admits only its owner; checking an ACL would take a crate this project
does not have (ADR-005), and most Windows users will borrow the `gh` or
`glab` login and store nothing.

**Where it differs.** The editor falls back to Notepad, `token_cmd` runs
through `cmd /C`, and the armed view's clock reads the time zone from the
registry rather than `date`.

**What proves it.** A `windows` CI job runs every test and a release build
on each change, and the release ships `pullpen_windows_amd64.zip` and
`pullpen_windows_arm64.zip`. `make check` stays POSIX shell: the budgets do
not depend on the platform and the Linux job holds them. Interactive use in
Windows Terminal is unverified until someone runs it.

## ADR-017 — A Merged section shows what each merge's pipeline did

Merging was the end of pullpen's job, but often not the end of the change: a
push to the base branch runs CI again, and that run is usually where deploys
live. The maintainer needs to tell from pullpen whether a deployment went
through, so a sixth default section, **Merged**, shows recently merged PRs
by what their merge commit's pipeline did.

**What counts as the pipeline.** Every check on the merge commit: on GitHub
its status-check rollup, read in the same search query; on GitLab the
pipeline for the merge commit's SHA (the head's, after a fast-forward), in
one more query per page of results. This is not the forge's deployment
records, whose use varies by repository; a deploy job in the pipeline is
covered either way. `o` on a row opens the pipeline.

**How long a row stays.** The query is `is:merged merged:7d`. A row leaves
24 hours after its merge, except a failure, which stays until `:clear`,
because time away is when a failed deploy most needs to still be showing.
`:clear` drops every finished row and remembers them, in `cleared.json` in
the state directory, so they do not return with the next sync.

**How it looks.** The armed view's grouped layout, with the groups `failed`,
`running` and `done`, the job that failed or is running in the right-hand
column, and a summary on the command line. A merged row anywhere shows its
pipeline in its glyph: ○ running, ✓ passed, ✗ failed. A pipeline seen running
rings the bell when it finishes. The design changed first (ADR-002): a
`mergedScreen`, the sixth tab, and pipeline glyphs on the armed view's
merged rows; the armed view's tab text shortened to fit six tabs.

**What it cost.** About 330 lines of code, which took it past the 9,000-line
cap; ADR-018 raises it.

## ADR-018 — The code cap rises to 10,000

ADR-013 set 9,000 lines with room for the publishing work. What came after
was the maintainer's: `u` to update a branch (ADR-015), Windows (ADR-016),
and the Merged section (ADR-017), which with its fixes from running it
against live forges took the code to 9,004 lines. Each was asked for, and
none is padding: the last trim available, folding two view checks into one,
was already made.

The cap rises to 10,000 lines. That is about 1,000 lines of room, roughly
three features the size of the Merged section, and the next raise needs its
own record again, measured as this one is.

## ADR-019 — A failure needs attention only while its branch is still failing

ADR-017 kept every failed post-merge pipeline until `:clear`. In use, the
failed group filled with failures that later merges had already fixed, and
the maintainer could not tell what needed them from what was stale.

**The rule.** For each base branch a merge failed on, pullpen asks for the
newest pipeline on that branch: on GitHub the checks on the branch's head
commit, on GitLab the newest pipeline for the ref. One lookup per repository
and branch, only for failures. While it fails, or cannot be read, the
failure **needs attention**: `main failing  deploy-production`. Once it
passes, the failure is history: it moves to done as `fixed  main passes
now`, with a dim ✗, and leaves a day after its merge like everything else.
This judges the branch, not the change: a later merge that fixed the build
also shipped this one.

**Resolving by hand.** `D` and `:dismiss` drop the selected or marked rows
and remember them, for a failure you have decided to live with.

**Knowing without looking.** The Merged tab counts only what needs
attention, the status line adds `✗ 1 broken` while any base branch is
failing, and the bell rings when a branch starts failing after a merge and
again when it passes. The design changed first (ADR-002): the group, the
reason, the dim glyph, `D` in the help overlay and the status line's count.

## ADR-020 — A PR's pipeline is a subscreen of the PR

The maintainer watches pre- and post-merge pipelines as they run, and needs
each job's output. The forges' web UIs show a pipeline as a page of its own,
cut off from the merge request that started it, which is the part they do
poorly. In pullpen, the pipeline belongs to the PR.

**Where it lives.** `w` (or `:watch`) on a PR, from any section or the
detail view, opens its pipeline screen. The PR's breadcrumb, title and branch
stay on top. Below them are its runs: after a merge, the merge commit's
pipeline on the base branch first, then the head's, which `zo` shows. Enter
on a job opens it, with its steps on the left and its log on the right. `q`
walks back one level, to wherever `w` was pressed.

**How live.** While the screen is open and anything in it is still running,
the runs are asked for every `sync.watch` (default 5s), and so is the open
job's log. Nothing is asked once everything has finished or the screen
closes. When a run finishes while watched, the PR's cached detail is dropped,
so the detail view's checks agree with it.

**What each forge can show.** On GitLab, every job's trace is readable while
it runs, and its tail is fetched with a byte range; the log's sections stand
in for steps. GitHub's API publishes a job's log only once the job finishes:
a running job's log answers 404, which was checked against a live Actions
run. Its steps' states are live in GraphQL, so a running GitHub job shows
which step it is on, says its log comes when it finishes, and `o` opens the
live page. A commit status from outside the forge's CI has only its link.

**Which pipeline is the branch's.** GitLab's `ref` filter also matches
merge-request pipelines: a release MR from the base branch shares its SHA
and ref, and its failed pipeline was being reported as the branch failing.
Every lookup of a branch's pipeline, here and in ADR-017's and ADR-019's,
asks for pushes only, and checks that the answer names the branch.

**How a log is shown.** SGR colours map onto the palette's tokens (red to
err, green to ok, yellow to warn, blue, cyan and magenta to acc), and no
background is painted. Every other escape is dropped. A redrawn line keeps
what a terminal would show. A log keeps its last 10,000 lines. Jobs show in
the forge's order, not failures first, because a pipeline reads as stages.

The design changed first (ADR-002): `pipelineScreen`, `jobScreen`, and `w` in
the help overlay.

## ADR-021 — The code cap rises to 11,000

The pipeline screen (ADR-020) measured 1,285 lines. Its own three modules
(the screens, the log parser, and the keys and refresh) are about 910 of
them; the rest is each forge's query and mapping, the core types and the
wiring. It took the code to 10,436 lines, past ADR-018's 10,000. The trims
available were taken: the line editor's keys and the backend's ask-and-send
became one helper each, which cost almost what they saved.

The cap rises to 11,000 lines, about 560 lines of room. As before, the next
raise needs its own record, measured as this one is.

## ADR-022 — The list shows the branch each PR merges into

The maintainer's MRs target `develop` or `main` depending on what they are,
and which one decides what a merge will do. The preview and the detail view
named the target, but the list, where the triage happens, did not.

A dim `target` column follows the repo, ten cells, on every row. As the list
narrows it folds second, after size: first would have hidden it beside the
preview on anything under 149 columns, where most of the triage happens.
Folding second, it shows beside the preview from 137 columns, and at 120
columns with the preview, or at 80, the layout is as before. Showing it only when it differs from the repo's
default branch was considered and not taken: it would cost a field on every
search, and a column that is sometimes blank reads as missing data.

The design changed first (ADR-002): `cols(w)` and the list header, and
`#809` targets `develop` in the fixtures so the goldens show the difference.

## ADR-023 — Deploys is a lane per branch; Merged is history; the defaults tighten

The maintainer wanted a place to go straight to when they want to see what
is deploying, and Merged was not it. Merged was doing two jobs: a record of
what merged, and a deploy monitor (ADR-017's groups, ADR-019's attention).
A deploy is about a branch and what is running on it now; Merged was
organised around PRs. Reviewing the tabs turned up more overlap.

**Deploys.** A new view, one lane per repo and branch that merges landed on
in the window. It shows the newest merge there that ran a pipeline, grouped
`deploying`, `failing` and `deployed`, with the branch's job on the right.
Enter opens the lane's pipeline screen (ADR-020). ADR-019's rule and its
signals move here unchanged: a lane fails while its branch's newest
pipeline still fails, and `D` dismisses one until a newer merge lands. The
tab, `✗ N broken` and the terminal title count failing lanes, and the bells
ring on a lane's changes. No forge query is added. Deploys and Merged share
`is:merged merged:7d`, and sections with the same query share one sync. A
direct push to a branch, with no PR, is not seen.

**Merged is history.** Every merge in the window, newest first, each with
its pipeline's glyph and outcome. A failure reads `failed`, whatever the
branch has done since: the record does not judge the branch, Deploys does.
ADR-017's 24-hour rule and groups leave it. `:clear` drops finished rows
that are not failing.

**Armed shows only what is armed.** Its "merged since" group listed the
same merges a second time, and only on GitHub, since GitLab's search never
returns merged MRs for `is:armed`. The news stays, as the summary line's
first part, pointing to Merged. A conflict still names the merge that
likely caused it, found in Merged.

**The defaults** become Mine · Review · Armed · Deploys · Merged · Open.

- **Agent-opened leaves.** It was always empty for anyone whose agents open
  PRs under their own login. `is:agent`, `[agents]` and the ◇ mark still
  work, and the commented example in `config.example.toml` brings the tab
  back.
- **All becomes Open, last.** It is the only tab that shows colleagues' PRs,
  and the new name says what it holds.

**Keys where they apply.** The grouped views' summary lines name that tab's
keys on the right, so `w` and `D` are not only in the help overlay.

**What it costs.** Users without `[[section]]`s get the new tabs on their
next start. Anyone with their own sections keeps them, but a `view =
"merged"` section loses ADR-017's groups. They add a Deploys section, the
four lines in `config.example.toml`, to get them back as lanes. About 120
lines of code.

The design changed first (ADR-002): the tab row, `deploysScreen`,
`mergedScreen` and `armedScreen`, and the list screens moved onto Open.

## ADR-024 — Every tab counts its rows; pullpen believes its own merges

Two things the maintainer met in the first hour of ADR-023.

**The Deploys tab read 0 with lanes on screen.** ADR-023 had it count
failing lanes, as Merged once counted what needed attention, and next to
"Deploys" a 0 reads as nothing deploying. Every tab now counts the rows it
shows. A failing branch is still on the status line (`✗ 1 broken`) and in
the terminal title, from any tab.

**A merged MR came back as open.** After a merge, pullpen removes the row and
syncs every section. A read that has not caught up with the write, on
gitlab.com within seconds, or a sync already under way, listed it as open
again. Pressing `M` a second time got GitLab's refusal of a merge that had
already happened. A PR pullpen merged or closed now stays out of open lists
until the forge reports it merged or closed, or for five minutes, whichever
comes first.

