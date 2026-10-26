# Working on pullpen

## Comments record intent, never history

A comment answers "why is this code shaped this way", so that changing it
badly is harder. It does not record what the code used to be. Git and
`docs/decisions.md` hold history.

**Keep** — the reason a choice is non-obvious:

```rust
// Width is the non-CJK figure: tui.js counts code points and every design
// glyph is one cell, so the CJK figure would shift every column after ○.
```

**Delete** — the same fact told as a story:

```rust
// We used to call width_cjk() here, which broke the golden tests on a
// Japanese locale for two days before anyone noticed.
```

The test: **would this comment still make sense to someone who had never
seen the previous version?** If it only lands as "we got this wrong once",
it belongs in a commit message, and the story belongs in the name of the test
that now catches it.

- A 30-line function does not need 40 lines of preamble. If the reasoning is
  that long, it is a record in `docs/decisions.md`.
- No self-narration. Comments describe the code, not the process that
  produced it.
- Prefer deleting a comment to writing one that restates the line below it.

## Read first, every session

`docs/north-star.md`, the record titles in `docs/decisions.md`, and
`docs/PLAN.md`. A decision already recorded is settled; a re-proposal is
answered with its number.

## Everything else

- The design wins on how things look, the plan on how they work (ADR-002).
  Change `design/tui.js` first, regenerate the goldens, then the Rust.
- Dependencies are the list in `docs/PLAN.md` §1. A new crate needs a record
  first (ADR-005).
- `make check` before every commit: fmt, clippy with warnings denied, tests,
  and the budgets. Do not argue with a budget; write a record to change it
  (ADR-004).
- Unit tests live in a sibling `tests.rs`, never inline.
- Conventional commits. `main` requires a pull request.
- Every user-facing claim names the test or CI job that proves it, or the
  claim goes.

## Instinct fences

Each is a failure that has actually shipped in agent-led work, so the
instinct is named before it fires.

- **Answer the ask.** Extra scope is proposed in prose, never shipped in the
  diff.
- **Fix in place.** No `_v2`/`_new`/`_old` parallel files; a replacement and
  the deletion it replaces travel in one commit.
- **Never discard an error** without a comment saying why ignoring it is
  correct. That includes `let _ =` on a `Result` and `.ok()` used to silence
  one.
- **No `unwrap()` or `expect()` outside tests** unless the comment beside it
  says why it cannot fail.
- **Never weaken or delete a test to go green.** A failing test is
  information; report what it told you.
- **Never claim something ran** without the command and its output. Anything
  unverified is labelled unverified.
- **No placeholders** without a named tracking issue. No `todo!()` in a
  merged milestone.
- **One diff, one intent.** Refactors and fixes ship separately.
- **Give the strongest counter-argument first** on any proposal, and build
  only what survives it.
- **Polish is not proof.** Only executed checks count.
- **Open the pull request when the work is done** and the checks are green,
  not before.
