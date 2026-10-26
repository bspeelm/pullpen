# Troubleshooting

Start with `pullpen config check`. It checks the config, each forge's sign-in,
section queries, key remaps and the palette file, and prints the fix for
anything that fails.

**The status line says `not signed in to github.com`, and the forge shows ✗.**
pullpen found no token. If you use `gh`, check it is installed and signed in
where you run pullpen: on an image-based system, a `gh` inside a toolbox is
not on the host. Otherwise run `pullpen setup` and give it a token.

**A section says 0, or fewer than you expect.** Its query may be narrower than
you think: its text is on the right of the tabs. A section such as *All*
searches what you chose to watch in setup; change it with `pullpen settings`.

**The status line says an org hides its PRs from classic tokens.** That org
refuses classic GitHub tokens. Run `pullpen setup` and use your `gh` login, or
make a fine-grained token.

**`M` refuses to merge.** The message names what blocks it, such as a failing
check, a conflict or a draft, and the key that helps.

**A merge or an approval fails.** The message is the forge's own reason.
`:messages` keeps the last 200.

**Something else.** The log is `~/.local/state/pullpen/pullpen.log`; it never
contains a token. Attach `pullpen config check` and `pullpen --version` to an
[issue](https://github.com/bspeelm/pullpen/issues).
