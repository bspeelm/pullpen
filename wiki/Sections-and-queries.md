# Sections and queries

Each tab is a section: a name, a short name for narrow terminals, and a query.
Five ship:

| section | query |
|---|---|
| Mine | `is:open author:@me` |
| Needs my review | `is:open review-requested:@me` |
| Agent-opened | `is:open is:agent` |
| Auto-merge armed | `is:armed` |
| All | `is:open` |

Replace them in `config.toml` (`pullpen settings`, then `e`):

```toml
[[section]]
name  = "Infra"
short = "Infra"
query = "is:open repo:acme/infra"
```

## The query syntax

One syntax for both forges. pullpen translates each term into the forge's own
search, and evaluates the rest itself.

| term | matches |
|---|---|
| `is:open` `is:merged` `is:closed` `is:draft` | state |
| `is:armed` `is:agent` `is:ready` `is:failing` `is:conflict` | evaluated by pullpen |
| `author:<login>` `author:@me` `author:@agents` | who opened it |
| `review-requested:@me` `reviewed-by:@me` | reviews |
| `repo:<owner/name>` `repo:acme/*` | a repo, or a glob |
| `org:<name>` `group:<name>` `forge:gh` `forge:gl` | where |
| `ci:pass` `ci:fail` `ci:running` | checks |
| `label:<name>` `base:<branch>` | labels, target branch |
| anything else | text in the title |

A term pullpen does not know is matched against titles, so a typo narrows the
list visibly rather than failing; `pullpen config check` points it out.

A query that names no person, repo, org or group would mean every PR on the
forge, so it searches what you chose to watch in setup instead.
