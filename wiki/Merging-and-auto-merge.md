# Merging and auto-merge

## Merging

`M` on a ready PR asks one key:

```
Merge #812 platform/runner → main with squash?   y/Enter yes   m merge commit   r rebase   n/Esc cancel
```

`y` merges with the method shown, which is the repo's own default unless you
set one in settings. `m` or `r` merges with that method instead. Any other key
cancels, and is not acted on.

If the PR is only waiting on CI or a review, `M` offers to arm auto-merge
instead. If the forge would refuse, because of failing checks, a conflict or
a draft, pullpen refuses first and names the key that helps.

## Behind the base

A repo can require a branch to be up to date with its base before merging.
A PR that is only behind says `behind main: u updates it`, and `M` refuses
with the same hint, since arming would wait for an update nobody makes. `u`
asks the forge to update it: GitHub merges the base into the branch, GitLab
rebases it. CI then runs again on the result.

## Auto-merge

`A` arms auto-merge, so the forge merges the PR by itself once it is ready,
or disarms it if it is armed. Neither needs confirming; both can be undone.
When the repo allows several methods and you have not set one, `A` asks which.

A PR that is already green is merged at once rather than armed, since nothing
is left to wait for.

## Batches

Space marks a row and moves on; `V` marks from the last mark to the cursor.
With marks, every action applies to all of them, after one confirmation.
Rows that cannot take the action are skipped and named:

```
◆ Armed 4 PRs   #812 merging now (already green) · skipped #1203: draft
```
