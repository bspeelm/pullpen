# The merged view

The Merged section shows the PRs merged in the last day by what happened
next: the pipeline that runs on the base branch after a merge, which is
where deploys usually are.

```
  needs attention  1
▌ gh ✗   #4815 Remove deprecated v1 webhooks handler      api      main failing  deploy-production   1h

  running  2
  gh ○   #2288 chore(deps): update dependency vite to v7… web      running  5/7 · deploy-staging     22m

  done  4
  gh ✗   #4802 Upgrade to Go 1.25                         api      fixed  main passes now            6h
```

- **needs attention**: its pipeline failed, and the base branch is still
  failing now. The job that failed is named. It stays until the branch
  passes again, or you dismiss it, however long you were away.
- **running**: how many jobs have passed, and the one running now.
- **done**: passed, no pipeline ran, or **fixed**: it failed, but the
  branch's newest pipeline passes, so a later change fixed it. These leave
  a day after the merge.

Whether a failure still matters is judged by the branch: pullpen asks for
the newest pipeline on it. A later merge that fixed the build also shipped
the change that broke it.

## Knowing something needs you

- The **Merged** tab counts only what needs attention.
- The status line shows `✗ 1 broken` from any tab while a branch is failing.
- The bell rings, and a desktop notification goes out if you turned them on,
  when a branch starts failing after a merge and again when it passes.

## Resolving

- `D` (or `:dismiss`) on a row, or on marked rows, resolves it by hand: it
  leaves and does not come back. Use it for a failure you have decided to
  live with.
- `:clear` drops every finished row at once.
- `o` opens the pipeline itself rather than the PR.

The glyph on a merged PR is its pipeline wherever it appears, the armed
view included: ○ running, ✓ passed, ✗ failed, a dim ✗ failed but fixed since,
● none.

The section's query is `is:merged merged:7d`, scoped like All to what you
watch; change it in `config.toml` like any section, with `view = "merged"`.
