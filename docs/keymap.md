# Keys

Generated from `crates/pullpen-ui/src/keymap/mod.rs` by `make keymap`; edit the table, not this file.
The same table drives the `?` overlay.

## move

| key | does |
|---|---|
| `j k` | down / up |
| `gg G` | top / bottom |
| `^d ^u` | half page |
| `/` | filter list |
| `n N` | next / prev match |
| `Esc` | clear, cancel |
| `Space` | mark row |
| `V` | mark range |
| `Enter` | open detail |

## act on selection

| key | does |
|---|---|
| `M` | merge… |
| `A` | arm / disarm auto |
| `a` | approve |
| `c` | comment |
| `d` | open diff |
| `o` | open in browser |
| `x` | close… |
| `u` | update branch |
| `r` | refresh |

## sections & views

| key | does |
|---|---|
| `h l` | prev / next section |
| `Tab S-Tab` | next / prev section |
| `1–N` | go to section |
| `p` | toggle preview |
| `:` | command line |
| `?` | this help |
| `q` | back / quit |

## detail view

| key | does |
|---|---|
| `h l` | focus pane |
| `n b` | next / prev file |
| `F` | all files |
| `]c [c` | next / prev hunk |
| `zo zc` | expand / fold |
| `q Esc` | back to list |

## command line

| key | does |
|---|---|
| `Tab S-Tab` | complete |
| `↑ ↓` | history |
| `^r #` | insert selected |
| `Enter` | run |
| `Esc` | cancel |

## confirm prompt

| key | does |
|---|---|
| `y Enter` | confirm |
| `n Esc` | cancel |
| `s m r` | pick method |

## more

| key | does |
|---|---|
| `R` | refresh every section |
| `^l` | redraw |
| `]f [f` | next / prev file, as vim spells it (detail) |
| `^w ^u` | delete word / line (command line) |
| `d` | zoom the diff (detail) |
| `j k ^d ^u gg G` | scroll the focused pane (detail) |

Any other key at a confirm prompt cancels it and is discarded.
