# pullpen — implementation plan

*where your PRs warm up until they're called*

A terminal client for reviewing, arming auto-merge on, and merging pull requests across **GitHub and GitLab** from one vim-style interface. Built for an agent-heavy workflow: agents open PRs, the human triages, arms, walks away, comes back to a clean list.

This document is the architecture and the road. It fixes the decisions that would otherwise stall work (stack, layout, module boundaries, order of milestones) and points at the design artifact for every visual question. The reasons behind the decisions are numbered records in [`decisions.md`](decisions.md); the aim is [`north-star.md`](north-star.md); the working rules are [`CLAUDE.md`](../CLAUDE.md). Where this plan and the design artifact disagree on how something *looks*, the design wins. Where they disagree on how something *works*, this plan wins.

---

## 0. Inputs you have

| Input | What it is | How to use it |
|---|---|---|
| `design/tui.js` | **The executable spec.** Pure JS, no DOM. `window.TUI.build(mode)` renders every screen as arrays of styled cells. | Port its layout functions 1:1. Use it to generate golden snapshots (§7). |
| `design/PR Review TUI.dc.html` | The written spec: keymap, command list, color tokens, glyph tiers, collapse table, interaction rules, forge mapping | Read the prose once; it is reproduced in condensed form in §4–§6 below. |
| This file | Architecture, milestones, acceptance criteria | Work top to bottom; do not skip the golden-test harness in M1. |
| `design/support.js` | The design tool's page runtime | Only needed to view the `.dc.html` in a browser. |

The design calls the tool `rq` in a few places. The name is **pullpen** everywhere: binary `pullpen`, config dir `~/.config/pullpen/`, env prefix `PULLPEN_`.

Things in `tui.js` that matter most, by function name:

- `cols(w)` — the column-collapse algorithm. Port exactly (§5.4).
- `stateG(p)` — readiness glyph precedence: merged → draft → conflict → failing → waiting → ready.
- `rowA`, `headA` — the chosen row treatment ("1a columns"). Treatments `rowB`/`rowC` are **not** built.
- `tabs`, `status` — section bar and status line layout.
- `fit`, `lr`, `overlay`, `M` — the span/cell primitives; your render layer needs the same four.
- `PAL`, `ASCII` — color tiers and glyph fallbacks.
- `build()` — every screen and every command-line state, with fixture data `P`, `H`, `ALL`.

---

## 1. Decisions

Each row is settled. The ones with a record are answered by its number: ADR-001 the language, ADR-005 the crate list.

| Decision | Choice | Why |
|---|---|---|
| Language / TUI | **Rust**, `ratatui` + `crossterm` | Single static binary, instant startup, cell-level rendering control the design needs. |
| Async | `tokio` (multi-thread), `reqwest` with `rustls` | Forge calls run in background tasks; UI thread never blocks. |
| GitHub API | **GraphQL** (`https://api.github.com/graphql`) via `reqwest` + `serde_json` (hand-written queries, no codegen) | Auto-merge (`enablePullRequestAutoMerge`), review decision, status check rollup and mergeability are only sane via GraphQL. REST only for `PATCH`/`PUT` merge and checkout refs. |
| GitLab API | **GraphQL** to read (`/api/graphql`), **REST v4** to write (`/api/v4`) | One GraphQL query per page carries pipeline, approvals and diff stats that REST needs three calls per MR for (ADR-008). Writes use the stable REST endpoints. |
| Auth | In order: `PULLPEN_GITHUB_TOKEN` / `PULLPEN_GITLAB_TOKEN`, then `GITHUB_TOKEN` / `GITLAB_TOKEN`, then a token `pullpen setup` stored in `credentials.toml` (0600), then `gh auth token` / `glab auth status -t`, then `config.toml` `token_cmd` | Zero setup for anyone already using gh/glab; one question for anyone else (ADR-012). |
| Config | TOML at `$XDG_CONFIG_HOME/pullpen/config.toml` (`%APPDATA%\pullpen\config.toml` on Windows, ADR-016), written by `pullpen setup` and `pullpen settings` (§6.6) or by hand | Design says YAML/TOML; TOML is the Rust norm. |
| Cache | `$XDG_CACHE_HOME/pullpen/` (`%LOCALAPPDATA%\pullpen\cache\` on Windows) — JSON per section, history file, messages log | Instant first paint from cache, then refresh. |
| Clipboard / browser | `open` crate for `o`; no clipboard in v1 | |
| Row treatment | **1a columns** (`rowA`) | Chosen in design review. |
| Vocabulary | "PR" everywhere; GitLab `!812` displayed `#812`; both spellings accepted in commands | Design §01, §07. |
| Mouse | Optional, additive: click row selects, click tab switches, wheel scrolls panes | Everything reachable by keyboard first. |
| License | MIT | |

Crates (pin latest stable at implementation time). This list is the dependency ceiling; adding a crate takes a record first (ADR-005): `ratatui`, `crossterm`, `tokio`, `reqwest` (rustls, json), `serde`, `serde_json`, `toml`, `clap`, `directories`, `thiserror`, `anyhow`, `tracing` + `tracing-subscriber` (file log only, never stdout), `unicode-width`, `open`, `strsim` (did-you-mean), `async-trait` (object-safe `Forge`), `insta` (snapshots, dev), `wiremock` (forge tests, dev).

---

## 2. Repository layout

```
pullpen/
├── Cargo.toml                 # workspace
├── design/                    # the unzipped design artifact, committed
├── crates/
│   ├── pullpen/                 # binary: main.rs, cli args, wiring
│   ├── pullpen-core/            # domain: Pr, Readiness, Section, Query, Action, Event (no I/O, no ratatui)
│   ├── pullpen-forge/           # Forge trait + github/ + gitlab/ + auth
│   ├── pullpen-ui/              # render: cells, spans, palette, glyphs, layout, views, keymap, command line
│   └── pullpen-fixtures/        # P/H/ALL data from tui.js as Rust, plus loader for golden files
├── tests/golden/              # text snapshots generated from tui.js (see §7)
├── scripts/gen-golden.mjs     # node script: runs design/tui.js, dumps screens as plain text + color-annotated text
├── Makefile                   # `make check`: fmt, clippy, tests, budgets
├── CLAUDE.md                  # the working rules
└── docs/
    ├── PLAN.md                # this file
    ├── north-star.md          # what pullpen is for
    ├── decisions.md           # numbered records of why
    ├── config.example.toml
    └── keymap.md              # generated from the keymap table in code
```

Unit tests live in a sibling `tests.rs` (`#[cfg(test)] mod tests;`), never inline in the file they test. That keeps the budget counter a filename filter rather than a Rust parser (ADR-004).

Dependency direction: `pullpen` → `pullpen-ui`, `pullpen-forge` → `pullpen-core`. `pullpen-ui` depends on `pullpen-core` only; it must render from fixtures with no network (`pullpen --fixtures`), which is how all UI work and all snapshot tests run.

---

## 3. Domain model (`pullpen-core`)

```rust
pub enum Forge { GitHub, GitLab }

pub struct PrId { pub forge: Forge, pub host: String, pub repo: String /* owner/name */, pub number: u64 }

pub struct Pr {
    pub id: PrId,
    pub title: String,
    pub author: Author,            // { login, is_agent: bool }
    pub head: String, pub base: String,
    pub additions: u32, pub deletions: u32, pub files: u32,
    pub draft: bool,
    pub checks: Checks,            // { required_total, required_passed, running, failed: Vec<CheckRun> }
    pub reviews: Reviews,          // { have: u32, need: u32, changes_requested: bool, pending: Vec<login> }
    pub mergeable: Mergeable,      // Clean | Conflict | Behind | Unknown
    pub auto_merge: Option<AutoMerge>, // { method, armed_by, armed_at }
    pub merge_train: Option<MergeTrainPos>, // GitLab only
    pub state: PrState,            // Open | Merged{at, method} | Closed
    pub updated_at: DateTime, pub created_at: DateTime,
    pub url: String,
    pub repo_default_method: MergeMethod, pub repo_allowed_methods: Vec<MergeMethod>,
}

pub enum MergeMethod { Squash, Merge, Rebase }

pub enum Readiness { Merged, Draft, Conflict, Failing, Waiting, Ready }   // glyph precedence order
pub fn readiness(pr: &Pr, caps: &Capabilities) -> Readiness;
// Ready iff: required checks pass && reviews.have >= need && !changes_requested && !draft
//   && (mergeable == Clean || (mergeable == Behind && !caps.requires_up_to_date))

pub enum Verdict { Ready, Waiting, Blocked }  // detail view label
pub fn verdict(pr: &Pr) -> (Verdict, String /* the sentence */);  // see §6.4 sentences

pub struct Section { pub name: String, pub short: String, pub query: Query, pub sort: Sort }
pub struct Query { /* parsed forge-neutral filter, see §6.1 */ }

pub enum Action { Merge{method: Option<MergeMethod>}, Arm{method: Option<MergeMethod>}, Disarm, Approve, Comment(String), Close, Checkout, Open, Diff{external: bool}, Refresh{all: bool} }

pub enum Event { Synced{section, prs}, SyncError{forge, host, msg, retry_in}, ActionDone{target, action, summary}, ActionFailed{target, action, reason, hint}, ArmedTransition{pr, from, to} }
```

Agent detection: `author.login` matches a configurable list; default `["claude", "codex", "renovate", "dependabot", "devin", "copilot", "github-actions"]` plus any login ending in `[bot]` or GitLab `bot` user type.

---

## 4. Forge layer (`pullpen-forge`)

```rust
#[async_trait]
pub trait Forge: Send + Sync {
    fn kind(&self) -> ForgeKind;
    async fn search(&self, q: &Query, cursor: Option<Cursor>) -> Result<Page<Pr>>;
    async fn get(&self, id: &PrId) -> Result<Pr>;
    async fn detail(&self, id: &PrId) -> Result<PrDetail>;   // description, checks list, conversation, files
    async fn diff(&self, id: &PrId) -> Result<String>;        // unified diff
    async fn merge(&self, id: &PrId, m: MergeMethod) -> Result<()>;
    async fn arm(&self, id: &PrId, m: MergeMethod) -> Result<()>;
    async fn disarm(&self, id: &PrId) -> Result<()>;
    async fn approve(&self, id: &PrId) -> Result<()>;
    async fn comment(&self, id: &PrId, body: &str) -> Result<()>;
    async fn close(&self, id: &PrId) -> Result<()>;
    fn capabilities(&self, repo: &str) -> Capabilities;      // allowed merge methods, has merge trains, requires up-to-date
}
```

### 4.1 Mapping (from design §13)

| pullpen | GitHub | GitLab |
|---|---|---|
| PR #812 | pull request #812 | merge request !812 |
| checks | check runs + commit statuses (`statusCheckRollup`; required-ness from each context's `isRequired(pullRequestNumber:)`) | pipeline jobs (`/merge_requests/:iid/pipelines`, head pipeline) |
| required reviews | `reviewDecision`, plus the count from REST `GET /repos/{o}/{r}/rules/branches/{base}` (the `pull_request` rule). Not `branchProtectionRule`, which only admins can read | approval rules (`/approvals`, `approvals_left`, `approved_by`) |
| auto-merge | `enablePullRequestAutoMerge` / `disablePullRequestAutoMerge` (GraphQL) | `PUT /merge?auto_merge=true` (16.5+; older: `merge_when_pipeline_succeeds=true`); disarm `POST /cancel_merge_when_pipeline_succeeds` |
| merge | `PUT /pulls/:n/merge` with `merge_method` | `PUT /merge_requests/:iid/merge` with `squash`, `should_remove_source_branch` |
| merge train | — | `merge_trains` endpoints; show position |
| approve | `POST /pulls/:n/reviews {event: APPROVE}` | `POST /merge_requests/:iid/approve` |
| conflict | `mergeable == CONFLICTING` | `has_conflicts` / `detailed_merge_status` |
| behind base | `mergeStateStatus == BEHIND` | `detailed_merge_status == need_rebase` |
| squash | `squash` | `squash: true` on merge |
| rebase merge | `rebase` | fast-forward merge method (project setting `merge_method = ff`) |

GitHub's search asks for the cheap fields of up to 30 PRs, then a second `nodes(ids:)` query per 10 asks for mergeability and check details. Computing mergeability is what made a single query slow enough for GitHub to answer 502. A required check that has not reported counts as pending ("Expected"), not absent.

### 4.2 Errors

Map forge errors to `ActionFailed{reason, hint}` where `hint` is the next useful key, e.g. `#2290 can't merge: e2e/chromium failing · 1 review required` + `d diff   o open in browser`. Rate limits and 5xx become `SyncError` with backoff (1s, 2s, 4s… cap 60s) and surface on the status line, not the command line.

### 4.3 Sync loop

One task per section. An open pipeline screen asks for its runs and log every `sync.watch` (default 5s) while anything runs (ADR-020). Interval `sync.interval` (default 60s), plus immediate refresh on `r`, after any action, and on focus-in (crossterm `FocusGained`). Each sync writes the section cache file. Cursor stability (design §13): keep cursor on the same `PrId` after refresh; if gone, same index.

---

## 5. UI layer (`pullpen-ui`)

### 5.1 Primitives — port from `tui.js`

| tui.js | Rust |
|---|---|
| `S(t, f, {k,b,u})` span | `Span { text, fg: Tok, bg: Option<Tok>, bold, underline }` with `Tok` enum = `Bg Surf Sel Fg Dim Faint Acc Ok Warn Err` |
| `fit(l, w, ell)` | `fit(&[Span], w, ellipsis) -> Vec<Span>`; trailing-space behavior matches JS exactly (ellipsis replaces last char, keeps right padding). `tui.js` counts code points and every design glyph is one cell, so width is `unicode-width`'s non-CJK figure, and a test asserts every glyph in every tier measures 1: the CJK figure makes ○ ◆ ● ✓ two cells in some locales and shifts every column |
| `lr(a, b, w)` | left/right justify in width |
| `overlay(base, x, top)` | cell-wise overwrite (for help overlay and wildmenu) |
| `M("{aB|text}")` | a tiny markup macro/parser for fixtures and messages; tags `f d x a g r y` colors, `B` bold, `U` underline, `C` cursor (fg on bg), `S` sel bg |
| `PAL.tc`, `PAL.c16` | `Palette::truecolor()`, `::c256()`, `::c16()`; selection via `COLORTERM`/`TERM`, overridable in config |
| `ASCII` map | `Glyphs::unicode()`, `::ascii()`, `::nerd()`; **widths never change between tiers** |

### 5.2 Color tokens (design §10)

| token | truecolor | 256 | 16-color |
|---|---|---|---|
| bg | never painted | | |
| surface | `#1e1f23` | 234 | none |
| selection | `#282b31` | 236 | none → bold row + `>` gutter |
| fg | `#cbc9c4` | 251 | default |
| dim | `#8a8984` | 245 | bright black |
| faint | `#505055` | 239 | bright black |
| accent | `#7cc4dc` | 116 | cyan |
| ok | `#94c98b` | 114 | green |
| warn | `#dcc07a` | 180 | yellow |
| err | `#e48a7a` | 174 | red |

Rules: semantic colors only on glyphs and single keywords, never whole rows. Only background fills: selection, surface, the verdict label, the mode block. `NO_COLOR` → glyphs + bold carry everything. `ui.accent` in config overrides accent only.

### 5.3 Glyphs (design §10)

| meaning | unicode | ascii | nerd (opt-in) |
|---|---|---|---|
| ready | ✓ | + | nf-oct-check |
| waiting | ○ | o | nf-oct-clock |
| failing | ✗ | x | nf-oct-x |
| conflict | ≠ | ! | nf-oct-alert |
| draft | ◌ | - | nf-oct-git_pull_request_draft |
| merged | ● | . | nf-oct-git_merge |
| armed | ◆ | * | nf-oct-rocket |
| agent | ◇ | ^ | nf-md-robot_outline |
| forge | `gh` `gl` | same | nf-dev-github / nf-dev-gitlab |
| cursor / mark | ▌ • | > + | same |
| pane rule / ellipsis | │ … | \| ~ | same |
| spinner | ⠋⠙⠹⠸ | \| / - \ | same |

### 5.4 List row — columns and collapse (port `cols(w)` verbatim)

Fixed widths in cells of the **list pane** (not the terminal): gutter 2, forge 3, state 2, armed 2, number 6 (right-aligned `#4821`), title ≥24 flexible, repo 17, target 10 (the branch it merges into, ADR-022), author 12 (`◇ name`), reviews 5 (`have/need`, `–` if none required), size 12 (`+412 −88`), age 4 (right-aligned `12m 3h 2d 1w`).

Collapse order as list width shrinks: 1 size dropped → 1a target dropped → 2 author shrinks to `◇` only → 3 repo shortens to name after slash (width 11) → 4 age dropped → 5 reviews dropped → 6 forge dropped → 7 repo dropped → title truncates below 24. Preview pane is 46 wide + 3 for ` │ `; it **auto-hides below 110 terminal columns** (`p` forces it back).

Header row uses faint lowercase labels (`#  title  repo  author  rev  +/−  age`). Selected row: accent `▌` gutter, title bold, selection bg; marked row: accent `•`. Filter mode: non-matching rows go faint, matched substring in title underlined accent.

### 5.5 Chrome

- **Tabs row** (row 0): two-cell indent; each section `Name count`; active in accent bold, others dim/faint; 4 spaces between (3 when `short`). Right side: the section's query, faint. Below 100 columns use `short` names.
- **Status line** (row H−2, surface bg): `  MODE   section  pos/count  [n marked]  ····  ◆ 7 armed   ✗ 3 failing   synced 40s   gh ✓  gl ✓ `. Mode block: fg `bg` on accent (NORMAL/COMMAND/FILTER) or warn (CONFIRM); **never red**. Background error replaces sync segment in red and flips the forge's ✓ to ✗. Below 100 columns counts drop words (`◆7 ✗3`).
- **Command line** (row H−1): always reserved, so nothing shifts when `:` opens. Holds command input, confirm prompts, results, errors. Results persist until the next key.

### 5.6 Views

1. **List** — tabs, header, rows, optional preview (right, 46 cells: title, meta line, verdict line, 4 facts `checks / reviews / conflicts / auto-merge`, branch, size, wrapped description, activity).
2. **Detail** — breadcrumb row (`Section › gh repo #n` left; `1 of 23   updated 12m ago` right), title, meta line, **verdict row** (` READY `/` WAITING `/` BLOCKED ` filled label + sentence), 2×2 fact grid, then two panes split 56/rest: overview (description, checks list with `+ 7 more passed  zo to expand`, conversation) │ diff (file header with `file 2/14 +120 −12`, hunks with old/new line numbers in faint, `+` ok `-` err). `h`/`l` focus; focused pane heading accent. `d` zooms diff full width.
3. **Armed section** — rows grouped `needs attention` / `waiting`; each row's right side is the *why* (`failing  e2e/chromium · 2 retries`, `review  @mara · 1 of 2`, `ci  6/9 running · ~4m left`, `merge train  position 2 of 3`). Command line shows the summary, pointing to where merges are listed, with the tab's keys on the right: `◆ 5 merged since 09:14 → Merged · 2 need attention · 5 still waiting      Enter detail   A disarm   w pipeline` (ADR-023).
   **Deploys section** (ADR-023, ADR-019) — one lane per repo and branch merges landed on in the window, showing the newest merge there that ran a pipeline, grouped `deploying` / `failing` / `deployed`. A lane fails while its branch's newest pipeline still fails (`main failing  deploy-production`); once that passes it is deployed (`fixed  main passes now`, dim ✗), and `D` dismisses one (`dismissed  deploy-production`) until a newer merge lands. Enter opens the lane's pipeline screen; `o` its page. The tab, the status line's `✗ 1 broken` and the terminal title count failing lanes. Summary `✗ 1 failing · 2 deploying · 2 deployed`, keys `Enter pipeline   D dismiss   o open`.
   **Merged section** (ADR-017, ADR-023) — every merge in the window, newest first, under one heading, each with its pipeline's glyph and outcome (`failed  deploy-production`, `running 2/4 · build`, `passed 6/6`, `fixed`, `no pipeline`); `:clear` drops the finished ones that are not failing; `w` and `o` open the pipeline. Summary `7 merged`, keys `w pipeline   :clear drop finished`.
   **Pipeline and job screens** (ADR-020) — subscreens of a PR, opened with `w`. The PR's breadcrumb, title and branch stay on top. The pipeline screen lists the runs (after merge, then before), each job with its stage, progress and time; the job screen shows steps │ log, following the tail. Refreshed every `sync.watch` while anything runs.
4. **Help overlay (?)** — base screen faded to faint; 96-wide surface block at x=12, y=5, no border; three-column key tables; typing filters; `?`/Esc closes.
5. **Wildmenu** — completion list overlays upward over the list rows above the command line, surface bg, selected item sel bg.

---

## 6. Behavior

### 6.1 Sections and query syntax

Config:

```toml
[[section]]
name  = "Needs my review"
short = "Review"
query = "is:open review-requested:@me"
sort  = "updated"          # updated | created | age | repo

[[section]]
name  = "Auto-merge armed"
short = "Armed"
query = "is:armed"
view  = "armed"            # the grouped lifecycle view

[[section]]
name  = "Deploys"
short = "Deploys"
query = "is:merged merged:7d"
view  = "deploys"          # one lane per branch merges landed on (ADR-023)
```

Query grammar (forge-neutral, translated per forge; unknown terms are free text matched on title):
`is:open|merged|closed|draft|armed|agent|ready|failing|conflict`, `author:<login|@me|@agents>`, `review-requested:@me`, `reviewed-by:@me`, `repo:<owner/name|glob>`, `org:<name>`, `group:<name>`, `forge:gh|gl`, `ci:pass|fail|running`, `label:<x>`, `base:<branch>`, free text.

Translation splits each query in two. Terms a forge's search understands go to the server; the rest (`is:armed`, `is:agent`, `is:ready`, `is:failing`, `is:conflict`, `ci:*`) are evaluated on the fetched `Pr`s. A query made only of client-side terms fetches a superset (`involves:@me` on GitHub, `scope=all` within configured groups on GitLab). GitHub search returns at most 1,000 results; a section that hits the cap says so on the status line. A query that names no person, repo, org or group would mean every PR on the forge, so it is scoped: on GitHub by the forge's `scope` (default `involves:@me`), on GitLab to its configured `groups`, or, with none, to the union of PRs you authored, review or are assigned.

`/` filter inside a section uses the same grammar plus free text. Defaults ship (ADR-023): Mine, Needs my review, Auto-merge armed, Deploys (`is:merged merged:7d`, view `deploys`), Merged (the same query, view `merged`), Open (`is:open`). Sections with the same query share one sync. An Agent-opened section (`is:open is:agent`) is one `[[section]]` away. `merged:<age>` limits to PRs merged within that long, like `7d` or `24h`.

### 6.2 Keymap (design §09) — implement as a table, generate `docs/keymap.md` and the `?` overlay from it

**Normal, everywhere:** `:` command line · `?` help · `q` back (quits from list) · `Esc` clear filter/marks/message · `h l` prev/next section (list) or pane focus (detail) · `Tab S-Tab` next/prev section · `1–9` section by position · `r R` refresh section/all · `^l` redraw.

**List:** `j k` · `gg G` · `^d ^u` · `/` filter, `n N` next/prev match · `Space` mark, `V` mark range · `Enter` detail · `p` toggle preview · `M` merge (confirm) · `x` close (confirm) · `u` update branch (ADR-015) · `D` dismiss a merged failure (ADR-019) · `A` arm/disarm · `a` approve · `c` comment · `d` diff · `o` browser.

**Detail:** `j k ^d ^u gg G` scroll focused pane · `h l` focus overview/diff · `n b` next/prev file (`]f [f` too), `F` the file list (ADR-014) · `]c [c` next/prev hunk · `zo zc` expand/fold · `d` zoom diff · `M A a c o x u` as in list · `q Esc` back, cursor kept.

**Command & confirm:** `Tab S-Tab` complete/cycle · `↑ ↓` history (prefix-filtered) · `^r #` insert selected PR ref · `^w ^u` delete word/line · `Enter Esc` run/cancel · confirm: `y Enter` yes, `n Esc` no, `s m r` yes-with-method, **any other key cancels and is discarded**.

`Space` marks the row and moves down, as ranger does; `V` marks every row from the last mark to the cursor. Multi-key sequences (`gg`, `]f`, `zo`) use a pending-key buffer with 500ms timeout. Keys are remappable under `[keys]` in config; defaults are the above.

### 6.3 Commands (design §07)

| command | behavior |
|---|---|
| `:merge [target] [squash\|merge\|rebase]` | Merge now. Confirms. If not ready, offers to arm instead. If forge would reject, refuses with reason + hint, no prompt. |
| `:automerge [target] [method\|off]` (alias `:am`) | Arm, re-arm with another method, or disarm. No confirm (not destructive). |
| `:approve [target]` | Approve. |
| `:comment [target] <text>` | Post comment; bare `:comment`/`c` opens `$EDITOR` (suspend TUI, restore after). |
| `:checkout [target]` | `git fetch` + checkout head branch in the clone found by `repo` → `[repos]` config map or cwd remote match; `checkout.worktree = true` uses `git worktree add`. |
| `:open [target]` | Browser. |
| `:diff [target] [--external]` | Detail with diff zoomed, or pipe to `diff.pager` (delta, difftastic). |
| `:close [target]` | Confirms. |
| `:update [target]` | Bring the branch up to date with its base: GitHub merges the base in, GitLab rebases. No confirm; asks once for several. `M` on a PR that is only behind refuses and names `u` (ADR-015). |
| `:refresh [all]` | |
| `:section <name>` | Completes names and short names. |
| `:messages` | Last 200 results/errors. |
| `:clear` | Drop merged rows from the armed view; in the Merged section, drop every finished row, remembered across sessions. |
| `:dismiss [target]` | In the Merged section, resolve a failure by hand: the row leaves and stays gone (`D`, ADR-019). |
| `:q` / `:qa` | Back-or-quit / quit from anywhere. |

**Targets:** omitted → selected row, or all marked rows if any. `#812` or `!812` resolves within the section; ambiguous across repos → completion asks for `repo#812`. Bare integer → displayed row index. With marks present the confirm prompt shows the count (`Arm auto-merge on **6 PRs** with squash?`).

**Confirm prompts** (on the command line, mode CONFIRM): `Merge #812 platform/runner → main with squash?   y/Enter yes   m merge commit   r rebase   n/Esc cancel`. Method prompt when arming and repo allows several and none configured: `Arm auto-merge on #377 with   s squash   m merge   r rebase      Enter squash   Esc cancel`.

**Messages:** success = `✓ Merged #812 into main   squash · source branch deleted` (green check, past tense, details dim). Error = `E  :approve #2290: you are the author…` (red). Unknown command = `E  not a command: :mrege   did you mean :merge?  Tab to fix` (strsim Jaro-Winkler ≥ 0.8). Target resolution is shown live on the right while typing: `→ platform/runner #812`.

**Batch:** marks + action → one confirm → sequential execution with per-row spinner; rows already green merge immediately; ineligible rows are skipped **and named** (`Armed 5 PRs   #812 merging now (already green) · skipped #1203: draft`).

### 6.4 Verdict sentences (detail view)

- READY (ok): `Nothing blocking. M merges with squash.`
- WAITING (warn), armed: `Squash-merges automatically when @mara approves. Nothing else is blocking.` / not armed: `Waiting on CI (4/9). A arms auto-merge.`
- BLOCKED (err): name the first blocker and the helping key: `e2e/chromium failing. d opens the diff, o the run.` / `Conflicts with main after #4815. :checkout to resolve.`

### 6.5 Armed lifecycle and notifications

armed → waiting (CI / reviews / merge train) → merged, or → needs attention (check failed, conflict, changes requested, forge disarmed it). Transitions **waiting → merged** and **waiting → needs attention** ring the bell (`\x07`) and send an OSC 9 notification (`\x1b]9;pullpen: #812 merged\x07`); both configurable under `[notify]`. The Armed section groups by this state and remembers "since last visit" using a timestamp in the cache.

---

### 6.6 Setup and settings

`pullpen setup` walks through what pullpen needs, as plain prompts that work
over SSH: which forges, each host, sign-in (use the `gh`/`glab` login, or
paste a token read with echo off, checked by asking the forge who it belongs
to), the orgs, repos or groups an unscoped list searches, merge preferences
and notifications, and local clones for `:checkout`. Every question has a
default, and running it again starts from the last answers. A first launch
with no config and a terminal runs it before the list opens.

`pullpen settings` lists every setting with its value; choosing one shows what
it means and what it takes, and a valid answer is saved at once. `s` re-runs
sign-in, `e` opens sections and key remaps in `$EDITOR` and then checks them.
`pullpen settings get|set <key> [value]` does the same from a script, and
`pullpen settings path` prints both files. Saving keeps the previous
`config.toml` as `config.toml.bak`, because the TOML writer drops comments.

## 7. Golden tests from the design artifact

The design renders deterministic screens from fixture data. Make that the regression suite.

1. `scripts/gen-golden.mjs`: load `design/tui.js` in Node with a stub `window`, call `TUI.build({})`, `TUI.build({ascii:true,c16:true})`, and for each screen (`sList`, `sFilter`, `s80`, `s80ascii`, `sDetail`, `sHelp`, `sArmed`, each `mergeFlow[i]`, `batch[i]`, `cmd[i]`) write two files: `name.txt` (plain text, each line padded to width) and `name.ansi.txt` (text with inline `{tok}` markers for fg/bg/bold/underline). Commit the output under `tests/golden/`.

   `build()` exposes only its browser-ready output, so the generator undoes three things `fin()` did. Rows with no content come back with no segments and are padded to the screen width. Glyphs arrive as their own segments and are merged back into runs of one style. Colours arrive as hex and are mapped back to tokens; the eleven truecolor values are distinct, so the mapping is exact. The 16-colour screens keep the resolved colour instead of a token, because `dim` and `faint` are both `#808080` there and the token cannot be recovered. `mergeFlow`, `batch` and `cmd` are row strips (rows at width W−1, then the status and command lines), not full screens, so the Rust harness renders them with a strip renderer.
2. `pullpen-fixtures` holds `P`, `H`, `ALL` and the per-screen state (selected index, marks, filter string, preview on/off, command-line contents) transcribed from `build()`.
3. `pullpen-ui` snapshot tests render each screen through the real renderer into a `ratatui::buffer::Buffer`, serialize with the same `{tok}` scheme, and `insta::assert_snapshot!` against the golden file. **The first milestone's exit criterion is these passing byte-for-byte**, which guarantees the collapse algorithm, truncation and chrome match the design.
4. Keep the generator; when a design tweak is needed, change `tui.js` first, regenerate, then make Rust match.

---

## 8. Milestones

Each milestone is a PR-sized chunk with a checkable exit. `pullpen --fixtures` (offline, fixture data, fake forge that succeeds/fails per a table) is available from M1 onward and is how everything through M2 is demoed.

### M0 — Scaffold
Workspace, crates, `make check` (fmt, clippy -D warnings, tests, budgets; ADR-004), CI running `make check` on linux and macOS, `north-star.md`, `decisions.md`, `CLAUDE.md`, `clap` args (`--config`, `--fixtures`, `--glyphs ascii|unicode|nerd`, `--colors 16|256|true`, `--section <name>`), tracing to `$XDG_STATE_HOME/pullpen/pullpen.log`, `design/` committed, `gen-golden.mjs` producing `tests/golden/*`.
**Exit:** `cargo run -- --fixtures` opens a blank TUI and `q` exits cleanly, terminal restored on panic (install a panic hook that leaves raw mode).

### M1 — Render core
Spans/fit/lr/overlay/markup; palette tiers + detection; glyph tiers; each frame wrapped in synchronized output (mode 2026); Kitty keyboard protocol where offered, so Tab and ^i differ; `cols(w)`; `rowA`/`headA`; tabs; status line; command line row; list view with preview; filter rendering; 80×24 and ASCII/16-color paths.
**Exit:** golden snapshots `sList`, `sFilter`, `s80`, `s80ascii`, `cmd[idle]` pass byte-for-byte. Resize is live.

### M2 — Input and modes
Modal key dispatcher with pending-sequence buffer; list navigation; sections (`h l Tab 1–9`); marks; `/` filter with `n N`, Enter-to-hide, Esc; `:` command line with parser, target resolution, wildmenu completion (commands, targets with glyph+title, section names), history (persisted), `^r`, `^w ^u`; CONFIRM mode with `y/Enter/n/Esc/s/m/r`-and-anything-else-cancels; messages (success/error/typo) and `:messages`; `?` overlay with filtering. All actions hit a fake forge.
**Exit:** golden snapshots for every `cmd[*]`, `mergeFlow[*]`, `batch[*]`, `sHelp` pass. Every key in §6.2 does something. `docs/keymap.md` generated from the table.

### M3 — Forge layer
`Forge` trait; auth resolution chain; GitHub GraphQL client (search → `Pr`, incl. `statusCheckRollup`, `reviewDecision`, `mergeable`, `mergeStateStatus`, `autoMergeRequest`, branch protection); GitLab REST client (search, approvals, head pipeline, `detailed_merge_status`, `merge_when_pipeline_succeeds`/`auto_merge`, merge trains); query translation; sync loop with cache, backoff, cursor stability; readiness computation; agent detection.
**Exit:** `pullpen` with a real token lists real PRs from both forges in the default sections; `wiremock` tests cover each endpoint's happy path and the error→`ActionFailed{reason,hint}` mapping. A test asserts a known token never appears in the cache, log or message history (ADR-006).

### M4 — Actions
merge / arm / disarm / approve / comment (incl. `$EDITOR` suspend/resume) / close / open / checkout (+worktree) / refresh, single and batch, with the §6.3 messages and the "not ready → offer to arm" and "would be rejected → refuse with reason" branches. Spinner on acting rows. Post-action refresh.
**Exit:** manual script in `docs/testing.md` completed on a scratch repo per forge: arm, merge, batch-arm with one draft skipped, approve-your-own-PR error renders correctly. Checkout fetches the PR's ref (`pull/N/head`, `merge-requests/N/head`), so forks work, and only ever fast-forwards an existing local branch.

### M5 — Detail view
Breadcrumb, verdict, fact grid, overview pane (description, checks with fold, conversation), diff pane (fetch unified diff, parse hunks, file/hunk navigation, fold), `h l` focus, `d` zoom, `--external` pager, all list actions available.
**Exit:** `sDetail` golden passes; `]f [f ]c [c zo zc` work on a 14-file fixture diff.

### M6 — Armed lifecycle
Armed view grouping (needs attention / waiting / merged since), *why* column, `:clear`, transition detection across syncs, bell + OSC 9, "since last visit" persistence.
**Exit:** `sArmed` golden passes; a fixture sync sequence (armed → ci running → merged) produces the right groups and exactly one notification.

### M7 — Polish and release
`NO_COLOR` (glyphs, bold and reverse video carry it); coloured underline for matches; mouse (click row, double-click opens detail, click tab, wheel); `[keys]` remapping by action name; `config.example.toml`; a release workflow building linux and macOS binaries for amd64 and arm64, every artifact and `checksums.txt` attested by `actions/attest-build-provenance` (ADR-011); release in two steps (`make release` opens the version-bump PR, `make release-tag` tags the merged commit); a Homebrew formula template, with the tap, Copr and AUR as their own issues; OSC 8 links on PR numbers are deferred (ADR-010); a README GIF waits on `vhs`; `pullpen --version`, `pullpen config check`.
**Exit:** fresh machine with only `gh auth login` done: `brew install <tap>/pullpen && pullpen` works.

---

## 9. Config reference (`docs/config.example.toml`)

```toml
[[forge]]                 # one table per host; repeat for GHES or self-managed GitLab
kind = "github"
host = "github.com"
# token_cmd = "gh auth token"
# scope = "involves:@me"  # what an unscoped query is narrowed to

[[forge]]
kind = "gitlab"
host = "gitlab.com"
# groups = ["platform"]   # searched when a query names no project, group or person
# token_cmd = "glab auth status -t 2>&1 | awk '/Token:/{print $2}'"

[ui]
glyphs = "unicode"        # unicode | ascii | nerd
colors = "auto"           # auto | 16 | 256 | true
accent = "#7cc4dc"
# palette = "~/.config/pullpen/palette.toml"   # any other palette, mapped onto the ten tokens (ADR-003)
preview = "auto"          # auto (hide < 110 cols) | on | off
short_names_below = 100

[sync]
interval = "60s"
on_focus = true
watch = "5s"              # how often an open pipeline screen asks again while a job runs (ADR-020)

[merge]
default_method = "repo"   # repo | squash | merge | rebase
delete_branch = true

[notify]
bell = true
osc9 = true

[agents]
logins = ["claude", "codex", "renovate", "dependabot", "devin", "copilot"]

[checkout]
worktree = false
[repos]                    # repo → local path, for :checkout
"acme/api" = "~/src/api"

[diff]
pager = "delta"

[keys]                     # optional remaps, e.g.
# "list.merge" = "M"

[[section]]
name  = "Mine"
short = "Mine"
query = "is:open author:@me"

[[section]]
name  = "Needs my review"
short = "Review"
query = "is:open review-requested:@me"

[[section]]
name  = "Auto-merge armed"
short = "Armed"
query = "is:armed"
view  = "armed"

[[section]]
name  = "Deploys"
short = "Deploys"
query = "is:merged merged:7d"
view  = "deploys"         # one lane per repo and branch merges landed on (ADR-023)

[[section]]
name  = "Merged"
short = "Merged"
query = "is:merged merged:7d"   # the same query as Deploys, so one search serves both
view  = "merged"

[[section]]
name  = "Open"
short = "Open"
query = "is:open"

# Not a default, for agents that open PRs under their own accounts:
# [[section]]
# name  = "Agent-opened"
# short = "Agent"
# query = "is:open is:agent"
```

---

## 10. Out of scope for v1

Settings UI, onboarding/auth screens, inline review comments on diff lines, creating PRs, Bitbucket/Gitea (keep the `Forge` trait clean so they can come later), row treatments 1b/1c. (Windows was here; ADR-016 brought it in.)

---

## 11. Working notes for the implementing agent

- Read `CLAUDE.md`, `north-star.md` and the record titles in `decisions.md` first, every session.
- Start by running `node scripts/gen-golden.mjs` and opening `tests/golden/sList.txt` — that is what done looks like.
- Never paint the terminal background. Never put semantic color on a whole row.
- The last two rows belong to the status line and command line at all times.
- When a decision is not covered here or in the design, prefer what vim or ranger does.
- One pull request per milestone; each milestone's exit criteria become its checklist. Conventional commits; `main` takes changes only through pull requests.
