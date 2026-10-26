# Where it runs

**Linux and macOS**, built and tested in CI on both on every change. Windows
is not built or tested.

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
