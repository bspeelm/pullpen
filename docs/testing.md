# Testing against a real forge

The automated suite covers every forge endpoint against recorded answers
(`wiremock`) and every screen against the design (`tests/golden/`). What it
cannot cover is a forge's own judgement: whether GitHub really enables
auto-merge on this PR, or GitLab really skips a draft. This script is that
check, run by hand before a release, once per forge.

**Status: not yet run.** It needs a scratch repository on each forge that
pullpen may write to.

## Setup

1. A scratch repository you own, with branch protection or approval rules
   requiring one review and one passing check, and auto-merge allowed.
2. Four PRs in it: two green, one with a running check, one draft.
3. `config.toml` with a section scoped to it:

   ```toml
   [[section]]
   name = "Scratch"
   query = "is:open repo:you/scratch"
   ```

## Script

Each step names what must appear. A step that shows anything else is a
failure, recorded with the message as shown.

1. **Arm.** Select the PR with the running check, press `A`, pick `s`.
   The row shows ◆; the line reads `◆ Armed #n   squash · waiting on CI …`;
   the forge's page shows auto-merge enabled.
2. **Merge.** Select a green PR, press `M`, then `y`. The spinner runs, then
   `✓ Merged #n into main   squash · source branch deleted`; the row leaves;
   the branch is gone on the forge.
3. **Batch arm with a draft.** Mark the remaining green PR and the draft
   with `Space`, press `A`, then `y`. The line ends
   `· skipped #n: draft`, and the green PR merges at once.
4. **Approve your own PR.** Select a PR you opened and press `a`. The line
   reads `E  :approve #n: …` with the forge's reason, and nothing changes.
5. **Disarm.** Select the armed PR from step 1, press `A`.
   `Auto-merge off for #n   A to re-arm`; the forge agrees.
6. **Checkout.** With the scratch repo cloned and listed under `[repos]`,
   run `:checkout #n`. `✓ Checked out #n   <branch> in <path>`, and
   `git -C <path> log -1` shows the PR's head commit.
