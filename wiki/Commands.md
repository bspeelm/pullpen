# Commands

## On the command line

| | |
|---|---|
| `pullpen` | open the queue |
| `pullpen --fixtures` | the design's sample data, offline |
| `pullpen --section <name>` | open on a section |
| `pullpen setup` | sign in and answer what pullpen needs |
| `pullpen settings` | the settings menu |
| `pullpen settings get <key>` | one setting's value |
| `pullpen settings set <key> <value>` | change one setting |
| `pullpen settings path` | where the config and credentials files are |
| `pullpen config check` | what works, and the fix for what does not |
| `pullpen --version` | the version |

## Inside pullpen

`:` opens a command line. `Tab` completes commands, PR numbers and section
names; `↑` `↓` walk history, filtered by what you have typed.

| | |
|---|---|
| `:merge [#n] [squash\|merge\|rebase]` | merge, after confirming |
| `:automerge [#n] [method\|off]` | arm, re-arm with another method, or disarm. `:am` for short |
| `:approve [#n]` | approve |
| `:comment [#n] <text>` | comment; bare `:comment` opens `$EDITOR` |
| `:checkout [#n]` | check out the head branch in your clone |
| `:open [#n]` | open in the browser |
| `:diff [#n]` | open the diff |
| `:close [#n]` | close without merging, after confirming |
| `:update [#n]` | bring the branch up to date with its base: GitHub merges the base in, GitLab rebases |
| `:refresh [all]` | fetch this section, or all |
| `:section <name>` | switch section |
| `:messages` | the last 200 results and errors |
| `:clear` | drop merged rows from the armed view; in the Merged section, every finished row |
| `:dismiss [#n]` | in the Merged section, resolve a failure by hand |
| `:q` `:qa` | back, or quit |

Without a number, a command acts on the selected row, or on every marked row.
`#812` and `!812` both work; a bare number is the row as displayed.
