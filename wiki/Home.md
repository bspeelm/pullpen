# pullpen

One terminal queue for pull requests across GitHub and GitLab. The
[README](https://github.com/bspeelm/pullpen) is the overview; these pages are
the detail. pullpen uses a few words precisely, and the rest of these pages
assume them.

## The words

| term | what it means here |
|---|---|
| **PR** | a pull request on GitHub or a merge request on GitLab. Both are shown as `#812`; `!812` is accepted too |
| **section** | one tab: a named query, such as *Needs my review*. Its count is how many PRs match |
| **readiness** | the one glyph per row: ✓ ready, ○ waiting, ✗ failing, ≠ conflict, ◌ draft, ● merged |
| **verdict** | the detail view's word for the same thing, READY, WAITING or BLOCKED, with a sentence saying what happens next |
| **armed** | auto-merge is on: the forge merges the PR by itself once it is ready. Shown as ◆ |
| **marks** | rows picked with Space. With marks present, every action applies to all of them |
| **agent** | a PR opened by an agent or bot, shown with ◇ before the author |
| **forge** | GitHub or GitLab, shown as `gh` or `gl` |
| **fixtures** | the design's sample data, `pullpen --fixtures`, for trying pullpen without a forge |

## Start here

- **[Your first session](Your-first-session)**: the screen, and the keys that matter on day one.
- **[Installing](Installing)**: downloading a release for your system, or building it.
- **[Setup and settings](Setup-and-settings)**: signing in and changing anything later.

## Doing the work

- **[Sections and queries](Sections-and-queries)**: the tabs, and the query syntax.
- **[Merging and auto-merge](Merging-and-auto-merge)**: confirming, methods, batches.
- **[The armed view](The-armed-view)**: coming back to what you armed.
- **[The merged view](The-merged-view)**: whether each merge's pipeline, and its deploy, went through.
- **[Watching pipelines](Watching-pipelines)**: a PR's jobs and their logs, live.
- **[Keys](Keys)** and **[Commands](Commands)**: everything there is.

## When something is off

- **[Troubleshooting](Troubleshooting)**: an empty list, a forge marked ✗, a refused merge.
- **[Signing in and security](Signing-in-and-security)**: where tokens come from and where one is kept.
- **[Configuration](Configuration)**: the file setup and settings write.
- **[Where it runs](Where-it-runs)**: platforms, forges, what is untested.

---

These pages live in [`wiki/`](https://github.com/bspeelm/pullpen/tree/main/wiki)
in the repository and are copied here when a change merges. An edit made here
is overwritten by the next copy; change the file instead.
