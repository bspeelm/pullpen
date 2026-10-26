# Installing

## From source

There is no release yet. You need Rust, then, from a clone:

```sh
make install-binary
```

That builds the release binary and copies it to `~/.local/bin/pullpen`, which
is on `PATH` on most systems and needs no root. `pullpen --version` confirms
it.

## With the first release

Each release will carry archives for Linux and macOS, on amd64 and arm64,
under fixed names so `releases/latest/download/` links keep working. Every
archive, and the `checksums.txt` that covers them, is signed by the workflow
that built it. To check one:

```sh
gh attestation verify pullpen_linux_amd64.tar.gz --repo bspeelm/pullpen
```

A Homebrew tap follows the first release.

## Removing it

Delete `~/.local/bin/pullpen`, and the directories listed under
[What it touches](https://github.com/bspeelm/pullpen#what-it-touches) if you
want your settings gone too.
