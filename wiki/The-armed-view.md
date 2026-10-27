# The armed view

The Auto-merge armed section groups armed PRs by what each needs:

- **needs attention**: a check failed, a conflict appeared, changes were
  requested, or the forge turned auto-merge off.
- **waiting**: what for, on the right: CI, a review from someone named, a
  place in the merge train.
The right-hand column says how long each has been that way. The command line
sums it up, starting with how many merged since you last left pullpen and
where they are listed, and names the keys for this tab on the right:
`◆ 5 merged since 09:14 → Merged · 2 need attention · 5 still waiting`. What
each merge then did on its branch is in [the deploys view](The-deploys-view).

When a waiting PR merges or needs attention, pullpen rings the bell and sends
a desktop notification (OSC 9), once. Outcomes you caused yourself, such as a
merge you pressed `M` for, ring nothing. Both are switched in settings.
