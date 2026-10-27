# Watching pipelines

`w` on a PR opens its pipeline. It works from any section, or from the
detail view. The screen stays tied to the PR: its title and branch stay on
top, so you always know which change a job belongs to. `q` goes back to
where you pressed `w`.

```
  Merged  ›  gl  platform/runner  #812  ›  pipeline                refreshed 3s ago

  Bump go toolchain to 1.25.3
  ◇ renovate   renovate/go-1.25.3 → main   merged 41m ago

  ○ after merge   main @ 3f2a91c   push              2 of 4 passed   running 2m 41s
  ✓  lint                 prepare     passed                                   38s
  ✓  go mod verify        prepare     passed                                   21s
▌ ○  build                build       running                              1m 40s
  ·  publish-image        deploy      queued

  ✓ before merge   9c01e7a   merge request                   4 of 4 passed   3m 05s
      zo to show its 4 jobs
```

- A merged PR shows the pipeline its merge started on the base branch first,
  which is where deploys run. Its own pipeline from before the merge is
  folded below; `zo` shows it and `zc` folds it.
- An open PR shows the pipeline on its head commit.
- The cursor starts on the first failed job, or else the first one still
  running.

While anything is still running, the screen asks the forge again every five
seconds (`watch` under `[sync]` in [Configuration](Configuration)). It
stops asking once everything has finished, or when you leave the screen.

## A job

Enter on a job opens it: its steps on the left, its log on the right.

- The log follows its end while the job runs. Scrolling up stops that, and
  `G` picks it back up.
- `h` moves to the steps, `j`/`k` choose one, and Enter jumps the log to
  where it starts. `l` goes back to the log.
- `/` searches the log, and `n`/`N` go to the next and previous match. `e`
  jumps to the next error line.
- `o` opens the job on the forge, and `r` asks for it again now.

Colours in the log come through as the palette's own: red, green, yellow and
cyan. A progress bar that redraws its line shows only its last state. A very
long log keeps its last 10,000 lines.

## What each forge shows

- **GitLab** shows a running job's log as it grows. Its steps are the
  log's sections: preparing the executor, getting sources, the script, and
  so on.
- **GitHub** shows a running job's steps and which one it is on, but its
  API publishes the log only once the job finishes. Until then the screen
  says so, and `o` opens the live log in the browser.
- A check from a service outside the forge's CI shows its state and a link.

What the screen draws is checked against the design by the `sPipeline` and
`sJob` golden tests. The refresh and the keys are covered by the
`app::pipeline` tests, and the log handling by `views::log`. Decided in
[ADR-020](https://github.com/bspeelm/pullpen/blob/main/docs/decisions.md).
