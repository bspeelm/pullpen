# The merged view

The Merged section shows the PRs merged in the last day by what happened
next: the pipeline that runs on the base branch after a merge, which is
where deploys usually are.

```
  failed  1
▌ gh ✗   #4815 Remove deprecated v1 webhooks handler      api      failed  deploy-production      1h

  running  2
  gh ○   #2288 chore(deps): update dependency vite to v7… web      running  5/7 · deploy-staging  22m
```

- **failed**: the job that failed is named on the right. A failure stays
  until `:clear`, however long you were away.
- **running**: how many jobs have passed, and the one running now. When it
  finishes, pullpen rings the bell, as it does for the armed view.
- **done**: passed, or no pipeline ran. These leave a day after the merge.

`o` opens the pipeline itself rather than the PR. `:clear` drops every
finished row and remembers it, so it does not come back. The glyph on a
merged PR is its pipeline wherever it appears, the armed view included:
○ running, ✓ passed, ✗ failed, ● none.

The pipeline is every check on the merge commit: on GitHub, the checks on
that commit; on GitLab, the pipeline for it. The section's query is
`is:merged merged:7d`, scoped like All to what you watch; change it in
`config.toml` like any section, with `view = "merged"`.
