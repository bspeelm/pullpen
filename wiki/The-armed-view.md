# The armed view

The Auto-merge armed section groups armed PRs by what each needs:

- **needs attention**: a check failed, a conflict appeared, changes were
  requested, or the forge turned auto-merge off.
- **waiting**: what for, on the right: CI, a review from someone named, a
  place in the merge train.
- **merged since** the last time you left pullpen, kept for the session.
  `:clear` drops them.

The right-hand column says how long each has been that way. The command line
sums it up: `◆ 4 merged since 09:14 · 2 need attention · 5 still waiting`.

When a waiting PR merges or needs attention, pullpen rings the bell and sends
a desktop notification (OSC 9), once. Outcomes you caused yourself, such as a
merge you pressed `M` for, ring nothing. Both are switched in settings.
