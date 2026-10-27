# The merged view

Merged is the record: every PR that merged this week, newest first, each
with what the pipeline its merge started did.

```
  last 7 days  7
▌ gh ○   #2288 chore(deps): update dependency vite to v7.1.4   web      running  5/7 · deploy-staging   22m
  gh ✗   #4815 Remove deprecated v1 webhooks handler           api      failed  deploy-production        1h
  gh ✓   #1199 Ledger: emit backfill progress metrics          billing  passed  6/6                      1h
  gl ●     #60 Fix broken links in runner install guide        docs     no pipeline                      5h
  gh ✗   #4802 Upgrade to Go 1.25                              api      fixed  main passes now           6h
```

The glyph is the pipeline: ○ running, ✓ passed, ✗ failed, a dim ✗ for
failed but fixed since, and ● when no pipeline ran. A failure reads
**failed** whatever the branch has done since. The record does not judge
the branch; [the deploys view](The-deploys-view) does, and that is where a
broken branch is acted on.

- `w` opens [the pipeline](Watching-pipelines) for a merge, and `o` opens it
  on the forge.
- Enter opens the PR, as in any list.
- `:clear` drops every finished row that is not failing, and they do not come
  back.

The section's query is `is:merged merged:7d`, scoped like Open to what you
watch. Change it in `config.toml` like any section, with `view = "merged"`.

Checked against the design by the `sMerged` golden test, and by the
`app::merged` tests. See ADR-017 and ADR-023 in
[decisions](https://github.com/bspeelm/pullpen/blob/main/docs/decisions.md).
