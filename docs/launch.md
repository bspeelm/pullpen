# Publishing on GitHub

How pullpen goes from the local branches to a public repository with
bothy's process around it. §2 is done: the repository exists, with one PR open. §3 is pending.

Decided already: the repository is **public** (`bspeelm/pullpen`), like
bothy. On a free plan the wiki, branch rulesets and CodeQL work only on
public repositories, and the scratch repository needs branch protection
before auto-merge can be tested.

## 1. Local work, on a `process` branch stacked on the newest milestone

**README in bothy's shape**, sections where they apply:

- a title and one line, the command, then the 80×24 screen from
  `tests/golden/s80.txt` as a text block, standing in for bothy's screenshot
- *What it does*, *Install* (from source and `make install-binary` until
  the first release, then the release archive and the tap),
  *Commands*, *The keys*, *Sections and queries*, *Merging and auto-merge*,
  *Coming back* (the armed view), *What it touches* (the three directories,
  and how tokens are handled), *Where it runs*, *Scope*,
  *Contributing*, *Authorship*, *Credits and licence*
- each section ends with a link into the wiki page that holds the detail,
  as bothy's do

**Wiki**, written in `wiki/` and reviewed through PRs like anything else:

- `Home.md`: a table of the terms (queue, section, PR, readiness glyph,
  verdict, armed, marks, forge, fixtures), *Start here*, an index of every
  page, and a footer saying UI edits are overwritten by the next sync
- `Your-first-session`, `Installing`, `Setup-and-settings`, `Commands`,
  `Keys`, `Sections-and-queries`, `Merging-and-auto-merge`,
  `The-armed-view`, `Configuration`, `Signing-in-and-security`,
  `Troubleshooting`, `Where-it-runs`

**Docs tests**, extending `crates/pullpen/tests/docs.rs` as bothy's
`docs_test.go` does: every wiki link resolves to a page in `wiki/`, `Home.md`
indexes every page, every wiki URL in the README names a page that exists.

**CI**, bothy's hallmarks in `.github/workflows/ci.yml`:

- `scope`: markdown-only changes skip the build jobs; the job is skipped, not
  the workflow, so required checks still report
- `check` (Linux) and `macos` as separate jobs, so the ruleset can name them
- `golden`: regenerate from `design/tui.js` and fail on any diff
- `one-palette`: the ADR-003 scan, asserting the filter matched a test
- `token`: the ADR-006 test, asserting it ran
- `release-build`: the release profile builds and `pullpen --version` runs,
  so the first release is not the first time
- every job that filters tests asserts the filter matched something;
  every pipe runs under `pipefail`

**Other workflows and files:**

- `wiki.yml`: mirrors `wiki/` into the wiki repository on merge to `main`,
  refusing to push an empty set (copied from bothy, with its comments)
- `outdated.yml`: weekly, `cargo update --workspace --dry-run`; opens or
  updates one tracking issue labelled `dependency-updates`, never a PR,
  never an automatic bump
- `.github/rulesets/main.json`: no deletion, no force-push, linear history,
  pull request required (squash or rebase), required checks `check`,
  `macos`, `golden`, `one-palette`, `token`, and CodeQL at high or above
- issue templates: a bug report asking for `pullpen --version` and
  `pullpen config check`; security reports routed to the advisory form

## 2. Creating the repository

1. `gh repo create bspeelm/pullpen --public`, wiki enabled, description from
   the README's first line.
2. Push the M0 commit as `main`. It is the only commit that lands without a
   pull request, because there is no base for one yet.
3. Push one branch with every commit after M0, one commit per milestone,
   and open **one** pull request for it. Stacked PRs, one per milestone,
   were tried first: the first was squash-merged and every one after it
   conflicted.
   Set the repository's merge settings as bothy's are: auto-merge allowed,
   branches deleted on merge, squash commits titled from the PR.
4. Apply the ruleset through the API from `.github/rulesets/main.json`.
5. Turn on CodeQL default setup. Rust is supported; if the API refuses, say
   so instead of assuming it is on.
6. The wiki repository does not exist until its first page is created in the
   web UI. Try pushing `wiki/` to it. If GitHub refuses, the maintainer
   creates one placeholder page by hand, and the push follows.
7. The PRs are left open for review. Nothing is merged without the
   maintainer.

## 3. The manual script against real forges (`docs/testing.md`)

**GitHub:** a public scratch repository, `bspeelm/pullpen-scratch`:

- a workflow with one check, `ci`, that passes at once on most branches and
  sleeps ten minutes on a branch named `slow-*`, so one PR stays waiting
- a ruleset requiring `ci`, which is what lets auto-merge be enabled at all;
  no required review, because the only account cannot approve its own PRs
- auto-merge allowed in the repository settings
- four PRs: two green, one `slow-*`, one draft
- then each step of `docs/testing.md` driven through pullpen, with what each
  step showed written into that file's status line
- offer to delete the repository afterwards

**GitLab:** cannot be run from this machine, which has no GitLab account or
token. It stays recorded as not run.
