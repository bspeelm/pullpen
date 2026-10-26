# pullpen

*where your PRs warm up until they're called*

One terminal queue for the pull requests you care about, across GitHub and
GitLab. Move with vim keys, read readiness from one glyph per row, merge or
arm auto-merge from the keyboard, walk away, come back to a clean list.

**Status: pre-release.** Everything in [`docs/PLAN.md`](docs/PLAN.md) through
M7 is built and tested. Not yet done: a run of [`docs/testing.md`](docs/testing.md)
against real repositories on each forge, a first release, and the Homebrew
tap. Real-forge behaviour has been checked read-only against GitHub only.

## Use

```sh
cargo build --release
./target/release/pullpen               # sign-in from gh or glab, or a token
./target/release/pullpen --fixtures    # the design's data, offline
./target/release/pullpen config check  # what works, and how to fix what doesn't
```

`?` lists every key; [`docs/keymap.md`](docs/keymap.md) is the same list.
Configuration is optional: [`docs/config.example.toml`](docs/config.example.toml).
pullpen never stores a token; it borrows one from the environment, `gh` or
`glab` (ADR-006).

## Where things are

- What it is for: [`docs/north-star.md`](docs/north-star.md)
- Why it is the way it is: [`docs/decisions.md`](docs/decisions.md)
- What it looks like: [`design/`](design/); open the `.dc.html` in a browser
- Contributing: [`CONTRIBUTING.md`](CONTRIBUTING.md)

MIT licensed.
