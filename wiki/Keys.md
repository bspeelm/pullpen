# Keys

The full table is
[`docs/keymap.md`](https://github.com/bspeelm/pullpen/blob/main/docs/keymap.md),
generated from the code, and `?` inside pullpen shows the same table.

The ones to know first:

| key | does |
|---|---|
| `j` `k` `gg` `G` `^d` `^u` | move |
| `h` `l` `Tab` `1`–`9` | sections |
| `Enter` | open the PR |
| `/` then `n` `N` | filter, next and previous match |
| `Space` `V` | mark a row, mark a range |
| `M` `A` `a` `c` `x` | merge, arm, approve, comment, close |
| `u` | update the branch from its base |
| `D` | in Deploys, dismiss a failing branch |
| `w` | the PR's pipeline, live: see [Watching pipelines](Watching-pipelines) |
| `o` `d` | browser, diff |
| `:` `?` `q` | command line, help, back |

In the detail view, `h` and `l` move between the overview and the diff,
`n` and `b` move to the next and previous file, `F` lists every file to
jump to, `]c` `[c` move between hunks, `zo` `zc` unfold and
fold the checks, and `d` widens the diff.

Single keys can be rebound by name in `config.toml`, under `[keys]`:
`merge = "m"` adds `m` for merging, and `M` keeps working.
