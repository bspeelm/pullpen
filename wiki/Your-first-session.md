# Your first session

Run `pullpen`. The first time, `pullpen setup` runs before anything else:
answer its questions and the queue opens. To look around with no forge at
all, `pullpen --fixtures` opens the design's sample data instead.

## The screen

The top row is the sections, one tab each, with how many PRs match. The
active one is the only coloured text; its query is on the right.

Each row is one PR: forge, readiness glyph, ◆ if armed, number, title, repo,
author (◇ for an agent), reviews as approved/needed, size and age. On a wide
terminal the preview on the right answers "safe to merge?" for the selected
row: the verdict, then checks, reviews, conflicts and auto-merge.

The second row from the bottom is the status line: the mode, where you are,
how many PRs are armed and failing, when it last synced, and each forge with
✓ or ✗. The bottom row is the command line, and holds prompts and results.

## The keys for day one

| key | does |
|---|---|
| `j` `k` | down, up |
| `h` `l` or `Tab` | previous, next section |
| `Enter` | open the PR: verdict, description, checks, conversation, diff; there, `n` and `b` step through files and `F` lists them |
| `M` | merge, after one key to confirm |
| `A` | arm auto-merge, or disarm it |
| `o` | open it in the browser |
| `/` | filter by title |
| `?` | every key |
| `q` | back, or quit |

[All the keys](Keys).

## Walking away

Arm what is only waiting on CI or a review with `A`, then leave. When you
come back, the Auto-merge armed section groups everything by outcome. See
[the armed view](The-armed-view).
