# Contributing

Most of what you need is already written down; this file says where.

**Why things are the way they are** is [`docs/decisions.md`](docs/decisions.md).
A change may contradict a recorded decision; name which one, so the reasoning
can be followed rather than rediscovered.

**The rules the code is written to** are in [`CLAUDE.md`](CLAUDE.md). Three
carry most of the weight:

- **`make check` before every commit.** Formatting, clippy with warnings
  denied, the tests, and the size budgets (ADR-004). The budgets are failing
  checks, and that is their job.
- **The design leads.** A visual change starts in `design/tui.js`, then
  `make golden`, then the Rust (ADR-002).
- **A bug fix ships with a test**, and the test name carries the story.

**Conventional commits**, and `main` requires a pull request.

A suspected security hole goes through [`SECURITY.md`](SECURITY.md) rather
than the tracker.
