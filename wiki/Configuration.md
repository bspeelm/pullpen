# Configuration

`pullpen setup` and `pullpen settings` write `~/.config/pullpen/config.toml`
(or under `$XDG_CONFIG_HOME`; on Windows `%APPDATA%\pullpen\config.toml`);
editing it by hand works as well. `pullpen settings path` prints where.
[`docs/config.example.toml`](https://github.com/bspeelm/pullpen/blob/main/docs/config.example.toml)
shows every key. The parts:

| table | holds |
|---|---|
| `[[forge]]` | `kind` (`github` or `gitlab`), `host`, and `scope` (GitHub) or `groups` (GitLab): what an unscoped section searches |
| `[[section]]` | `name`, `short`, `query`, and optionally `sort`, `view = "armed"`, `"deploys"` or `"merged"`, and `counts` |
| `[merge]` | `default_method` (`repo`, `squash`, `merge`, `rebase`) and `delete_branch` |
| `[notify]` | `bell`, and `osc9` for desktop notifications |
| `[sync]` | `interval`, `on_focus`, and `watch`, how often an open pipeline screen asks again (default `5s`) |
| `[ui]` | `glyphs`, `colors`, `accent`, `preview`, `palette` |
| `[repos]` and `[checkout]` | clone paths for `:checkout`, and `worktree` |
| `[keys]` | extra keys for actions, by name |

`pullpen --config <file>` reads another file for one run.
`pullpen config check` says what works and what to change.

## Your own palette

`ui.palette` names a file of ten colours, one per token: `bg`, `surf`, `sel`,
`fg`, `dim`, `faint`, `acc`, `ok`, `warn`, `err`, each as `name = "#rrggbb"`.
All ten, or pullpen keeps its own.
