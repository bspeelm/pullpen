# Installing

## From a release

Each release carries an archive for every system, under fixed names, so
`releases/latest/download/` links keep working:

| | amd64 | arm64 |
|---|---|---|
| Linux | `pullpen_linux_amd64.tar.gz` | `pullpen_linux_arm64.tar.gz` |
| macOS | `pullpen_darwin_amd64.tar.gz` | `pullpen_darwin_arm64.tar.gz` |
| Windows | `pullpen_windows_amd64.zip` | `pullpen_windows_arm64.zip` |

On Linux or macOS:

```sh
curl -LO https://github.com/lampseen/pullpen/releases/latest/download/pullpen_linux_amd64.tar.gz
tar -xzf pullpen_linux_amd64.tar.gz ./pullpen
mv pullpen ~/.local/bin/
```

Use the archive name for your system: `darwin_arm64` on an Apple silicon
Mac, `darwin_amd64` on an Intel one.

**macOS** may refuse to open it, saying the developer cannot be verified:
the binary is not notarised by Apple. Downloading with `curl` as above
avoids that. If you used a browser, clear the flag it set:

```sh
xattr -d com.apple.quarantine ~/.local/bin/pullpen
```

**Windows:** unzip it, put `pullpen.exe` in a folder on your `PATH`, and run
`pullpen` from Windows Terminal. The binary is not signed by Microsoft, so
SmartScreen may warn; *More info*, then *Run anyway*, or once in PowerShell:

```powershell
Unblock-File pullpen.exe
```

Every archive, and the `checksums.txt` that covers them, is signed by the
workflow that built it. To check one:

```sh
gh attestation verify pullpen_linux_amd64.tar.gz --repo lampseen/pullpen
```

A Homebrew tap is to follow.

## From source

You need Rust, then, from a clone:

```sh
make install-binary
```

That builds the release binary and copies it to `~/.local/bin/pullpen`, which
is on `PATH` on most systems and needs no root. On Windows, run
`cargo build --release` and copy `target\release\pullpen.exe` instead.
`pullpen --version` confirms it.

## Removing it

Delete the binary, and the directories listed under
[What it touches](https://github.com/lampseen/pullpen#what-it-touches) if you
want your settings gone too.
