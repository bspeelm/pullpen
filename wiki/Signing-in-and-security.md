# Signing in and security

## Where a token comes from

In order, the first that exists wins:

1. `PULLPEN_GITHUB_TOKEN` or `PULLPEN_GITLAB_TOKEN`, then `GITHUB_TOKEN` or
   `GITLAB_TOKEN`
2. a token you gave `pullpen setup`
3. your `gh` or `glab` login: pullpen runs `gh auth token` and uses what it
   prints
4. `token_cmd` in the config

pullpen always talks to the forge's API itself; the CLI only lends the token.

## The token setup keeps

`~/.config/pullpen/credentials.toml`, readable only by you. It is written to a
new owner-only file and renamed into place, so it never exists with wider
permissions. If anything makes it readable by others, pullpen refuses to use
it and prints the `chmod` that fixes it. It is kept apart from `config.toml`
so that file can be shared.

Setup reads a token with echo off. pullpen never writes one to the log, the
cache, the history or the screen, and a test holds it to that.

## Which token

The `gh` or `glab` login is the simplest, and works where pasted tokens do
not: some organisations refuse classic GitHub tokens (`ghp_…`), and their PRs
then do not show. pullpen says so on the status line when it happens. A
fine-grained token needs read and write on Pull requests and Contents for the
repos you watch.

## Reporting a security problem

Privately, through
[the advisory form](https://github.com/bspeelm/pullpen/security/advisories/new),
not the issue tracker.
