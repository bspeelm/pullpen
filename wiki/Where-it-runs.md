# Where it runs

**Linux, macOS and Windows**, built and tested in CI on all three on every
change; releases carry amd64 and arm64 builds. On Windows, nobody has yet run
it interactively; use Windows Terminal, which understands the escape codes
pullpen draws with. A report is welcome.

Any terminal works. Truecolor, 256 colours, 16 colours and `NO_COLOR` are all
handled, and ASCII glyphs are a setting. Terminals with the Kitty keyboard
protocol (Ghostty, Kitty, WezTerm) tell `Tab` from `^i`; synchronized output
stops flicker where supported.

**GitHub** has been exercised against real repositories: listing, details,
diffs, setup and sign-in.

**GitLab** has been tested only against recorded answers, with every field
pullpen reads checked against the gitlab.com schema. No GitLab account has
run it yet. A report from one is welcome.

**GitHub Enterprise and self-managed GitLab** use the same code with another
host, set in setup, and are untested.
