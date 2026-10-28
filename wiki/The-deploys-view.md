# The deploys view

Deploys answers one question: what is deploying right now, and did it work?
Each row is a lane, meaning a repo and a branch your merges landed on this
week, like `app-site develop` or `api main`. A lane shows the newest merge on
that branch that ran a pipeline, and what that pipeline is doing.

```
  deploying  2
▌ gh ○ web         main      #2288 chore(deps): update dependency vite…  running  5/7 · deploy-staging   22m
  gl ○ runner      main       #812 Bump go toolchain to 1.25.3           running  2/4 · build            41m

  failing  1
  gh ✗ api         main      #4815 Remove deprecated v1 webhooks handler main failing  deploy-production  1h

  deployed  2
  gh ✓ billing     main      #1199 Ledger: emit backfill progress metric… passed  6/6                    1h
  gh ✓ api         develop   #4811 Split payments service config by env… passed  11/11                  3h
```

- **deploying**: the pipeline is running. The right side says how many jobs
  have passed and which one is running.
- **failing**: the pipeline failed and the branch's newest pipeline still
  fails. The job that failed is named.
- **deployed**: it passed. Or it failed and a later change fixed the branch
  (**fixed**), or you dismissed it (**dismissed**).

Enter on a lane opens [its pipeline](Watching-pipelines), the run that
merge started on the branch, with every job and its log. `o` opens the
pipeline on the forge instead.

## Knowing something needs you

- The **Deploys** tab counts what is deploying right now. A deployed lane
  needs nobody, and a failing one is counted below.
- The status line shows `✗ 1 broken` from any tab while a branch is failing.
- The terminal's tab title reads `pullpen ✗1`, so it shows from another
  terminal tab too.
- The bell rings, and a desktop notification goes out if you turned them on,
  when a branch starts failing after a merge and again when it passes.

Whether a failure still matters is judged by the branch: pullpen asks for
its newest pipeline. A later merge that fixed the build also shipped the
change that broke it.

## Resolving by hand

`D` (or `:dismiss`) on a failing lane resolves it: it moves to deployed as
**dismissed** and nothing rings. It stays that way until a newer merge lands
on the branch. Use it for a failure you have decided to live with.

## What it does not see

A lane is built from merges, so a push straight to the branch with no PR in
it does not show. Deploys and [Merged](The-merged-view) share one query,
`is:merged merged:7d`. Change it on both to look further back.

What the view draws is checked against the design by the `sDeploys` golden
test, and its behaviour by the `app::deploys` tests. Decided in
[ADR-023](https://github.com/lampseen/pullpen/blob/main/docs/decisions.md).
