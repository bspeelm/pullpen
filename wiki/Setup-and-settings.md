# Setup and settings

## pullpen setup

Setup asks, in order:

1. **Which forges** you use: GitHub, GitLab, or both, and each host if you
   run your own.
2. **How to sign in.** If you are signed in to `gh` or `glab`, setup offers
   that login and stores nothing. Otherwise it gives you the page to make a
   token, reads it with echo off, and asks the forge whose it is before
   keeping it. See [signing in](Signing-in-and-security).
3. **Which PRs to watch**: orgs, users or repos on GitHub, groups on GitLab.
   This is what sections such as *All* search. Blank means only PRs that
   involve you.
4. **How to merge**: the method `M` and `A` use, whether to delete the branch
   afterwards, and whether to ring the bell or send a desktop notification.
5. **Where your clones are**, for `:checkout`, and whether to check out into
   a new worktree.

Every question shows its default in brackets; Enter takes it. Running setup
again starts from your last answers.

## pullpen settings

`pullpen settings` lists every setting with its current value. Type its
number, read what it means and what it takes, enter the new value. A value
pullpen could not read back is refused, with what is allowed. Each change is
saved at once.

`s` runs sign-in again. `e` opens the config in `$EDITOR` for the two things
that are lists, sections and key remaps, then checks what you wrote.

From a script:

```sh
pullpen settings get merge.default_method
pullpen settings set merge.default_method squash
pullpen settings path
```

Saving keeps the previous file as `config.toml.bak`, because comments you
wrote by hand do not survive a save.
