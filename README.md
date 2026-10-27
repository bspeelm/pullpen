# pullpen

One terminal queue for the pull requests you care about, across GitHub and
GitLab. Read readiness from one glyph per row, merge or arm auto-merge from
the keyboard, walk away, come back to a clean list.

```sh
pullpen
```

```
  Mine 6   Review 4   Armed 7   Deploys 1   Merged 7   Open 96

             # title                     repo               author    rev   age
  gh ○   #4823 Rate limit per API key…   acme/api           mara      0/1   20m
▌ gh ○ ◆ #4821 Migrate invoices to led…  acme/api         ◇ claude    1/2    3h
  gl ✓    #812 Bump go toolchain to 1.…  platform/runner  ◇ renovate  1/1   41m
  gh ✓   #2291 New billing settings la…  acme/web           jkoh      1/1    1h
  gh ✗ ◆ #2290 Fix hydration mismatch…   acme/web         ◇ codex     0/1    5h
  gh ○    #377 Rotate staging TLS cert…  acme/infra       ◇ claude    0/1   12m
```

A bullpen is where relievers warm up until they are called. This is where
PRs wait until they are ready.

## What it does

pullpen lists the PRs you care about from every forge you use, sorted into
sections you define as queries: yours, the ones waiting on your review, the
ones agents opened, the ones armed to merge. One glyph per row says whether
each is safe to merge: ✓ ready, ○ waiting, ✗ failing, ≠ conflict, ◌ draft.

Act on them without leaving: merge, arm auto-merge, approve, comment, close,
check out. Anything that cannot be undone asks one key to confirm. Mark rows
and act on all of them at once; those that cannot take the action are
skipped and named.

Arm a batch, walk away, and come back to the Auto-merge armed section grouped
by outcome: what needs you, and what is still waiting and why, with how many
merged while you were gone. The bell rings when one merges or needs attention.
[Your first session](https://github.com/bspeelm/pullpen/wiki/Your-first-session).

## Install

Download the archive for your system from the
[latest release](https://github.com/bspeelm/pullpen/releases/latest): Linux
and macOS as `.tar.gz`, Windows as `.zip`, each for amd64 and arm64, and each
signed by the workflow that built it. Put `pullpen` (`pullpen.exe` on
Windows) somewhere on your `PATH` and run it. The binaries are not
code-signed by Apple or Microsoft, so macOS and Windows may warn the first
time; [Installing](https://github.com/bspeelm/pullpen/wiki/Installing) says
what to do. With no settings yet, it starts
`pullpen setup` first.

From a clone, with Rust installed, `make install-binary` builds it into
`~/.local/bin/pullpen`. A Homebrew tap is to follow.
[Installing](https://github.com/bspeelm/pullpen/wiki/Installing).

## Setting it up

`pullpen setup` asks what pullpen needs: which forges, how to sign in to each,
which orgs, repos or groups to watch, how to merge, and where your clones are.
Every question has a default. Run it again any time; it starts from your last
answers.

`pullpen settings` lists every setting with its value; pick one, change it, it
is saved at once. `pullpen settings set <key> <value>` does the same from a
script.
[Setup and settings](https://github.com/bspeelm/pullpen/wiki/Setup-and-settings).

## Commands

| | |
|---|---|
| `pullpen` | open the queue |
| `pullpen setup` | sign in, and answer what pullpen needs to know |
| `pullpen settings` | see and change any setting |
| `pullpen config check` | what works, and what to type to fix what does not |

Inside, `:` opens a command line with completion, and `?` shows every key.
[All commands](https://github.com/bspeelm/pullpen/wiki/Commands) ·
[every key](https://github.com/bspeelm/pullpen/wiki/Keys)

## Sections are queries

Each tab is a query in one syntax that works on both forges:
`is:open review-requested:@me`, `is:open is:agent`, `repo:acme/api ci:fail`.
Six ship by default, and you can replace them.
[Sections and queries](https://github.com/bspeelm/pullpen/wiki/Sections-and-queries).

## Merging and auto-merge

`M` merges, after one key to confirm, using each repo's own default method
unless you set another; `s`, `m` or `r` at the prompt picks one for that merge.
If a PR is only waiting, `M` offers to arm auto-merge instead. If the forge
would refuse, pullpen refuses first and says why. `A` arms or disarms.
After a merge, the Deploys section shows each branch your merges landed on:
what is deploying now, what is failing, and what deployed, so you can see a
deploy go through or fail. A failing branch needs attention until it passes
again, and the status line says so from any tab. Merged keeps the week's
merges as a record. `w` on any PR, or Enter on a lane, opens its pipeline,
refreshed every few seconds while it runs. Enter on a job shows its steps
and its log.
[Merging and auto-merge](https://github.com/bspeelm/pullpen/wiki/Merging-and-auto-merge) ·
[the armed view](https://github.com/bspeelm/pullpen/wiki/The-armed-view) ·
[the deploys view](https://github.com/bspeelm/pullpen/wiki/The-deploys-view) ·
[the merged view](https://github.com/bspeelm/pullpen/wiki/The-merged-view) ·
[watching pipelines](https://github.com/bspeelm/pullpen/wiki/Watching-pipelines)

## What it touches

```
~/.config/pullpen/config.toml        your settings
~/.config/pullpen/credentials.toml   a token you gave setup, readable only by you
~/.cache/pullpen/                    the last list, so it opens instantly
~/.local/state/pullpen/              its log, command history, last visit, cleared rows
```

On Windows, the first two are in `%APPDATA%\pullpen\` and the others in
`%LOCALAPPDATA%\pullpen\cache\` and `state\`.

Nothing else. If you are signed in to `gh` or `glab`, pullpen borrows that
login and stores no token at all. A token never reaches the log, the cache or
the screen.
[Signing in and security](https://github.com/bspeelm/pullpen/wiki/Signing-in-and-security).

## Where it runs

Linux, macOS and Windows, built and tested in CI on all three; so far only
Linux has been used interactively. GitHub has been exercised against real
repositories, merging and arming included. GitLab has listed real merge
requests and their pipelines, but its merge, arm and approve have run only
against recorded answers.
[Where it runs](https://github.com/bspeelm/pullpen/wiki/Where-it-runs).

## Scope

pullpen lists, merges, arms, approves, comments, closes and checks out. It
does not create PRs, review code line by line, or manage branches; the forge's
own page does those well, and `o` opens it. It runs no background service and
sends no telemetry.

## Contributing

The aims are in [`docs/north-star.md`](docs/north-star.md), the reasoning
behind each decision in [`docs/decisions.md`](docs/decisions.md), and the
plan in [`docs/PLAN.md`](docs/PLAN.md). The rules the code is written to are
in [`CLAUDE.md`](CLAUDE.md), and [`CONTRIBUTING.md`](CONTRIBUTING.md) says
where everything else is.

## Authorship

pullpen was built in the shape of [bothy](https://github.com/bspeelm/bothy):
decisions recorded and numbered, the code held to budgets that fail the
build, and every claim in the documentation checked by a test.

The decisions are the maintainer's; Claude planned and implemented within
them, from a design made in Claude Design (`design/`), whose screens the
tests compare against byte for byte. Where the design contradicted itself,
the correction is recorded rather than made quietly.

## Credits and licence

MIT, see [`LICENSE`](LICENSE). The palette is the design's own.
