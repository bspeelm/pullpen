# North star

What pullpen is, so that every milestone can be judged by whether it gets it
closer. The reasons things are the way they are live in
[`decisions.md`](decisions.md); the architecture and the road are in
[`PLAN.md`](PLAN.md).

## The invariant

pullpen exists to produce one thing: **a single clean queue of the pull
requests you care about, across GitHub and GitLab, that you can clear without
opening a browser.**

The loop it serves is the one an agent-heavy week produces. Agents open PRs.
You triage them, arm auto-merge on the ones that only need time, and walk
away. When you come back, the queue tells you what merged, what needs you,
and what is still waiting and why.

Every screen answers one question: *is this safe to merge, and if not, what
is it waiting for?* The state glyph answers it in one cell, the preview in a
paragraph, and the detail view's verdict in one word and a sentence. A feature
that does not help answer that question, or act on the answer, does not
belong here.

## What that rules in

- Both forges in one list, with one vocabulary ("PR", `#812`). GitLab's
  differences are translated, not exposed.
- Acting from the keyboard: merge, arm, disarm, approve, comment, close,
  checkout, singly or in batches, each confirmed on the command line when it
  cannot be undone.
- Coming back to a clean list. The armed view groups by outcome, and a
  transition that needs you rings the bell.
- Degrading honestly. Sixteen colours and ASCII glyphs carry the same
  information as truecolor and Unicode. A forge that is down says so on the
  status line instead of showing a stale list as if it were fresh.

## What that rules out

- Writing code review comments on diff lines, creating PRs, or managing
  branches beyond checking one out. The forge's web UI does those well, and
  `o` is one key away.
- Storing credentials (ADR-006), and any settings or onboarding UI. The
  config file is the interface.
- Forges beyond GitHub and GitLab in v1. The `Forge` trait stays clean so
  that adding one later is additive.

## What it costs

Stated plainly, because a document that lists only benefits is a sales
document.

**Two forges means translating everything twice.** Readiness, required
reviews, auto-merge and merge trains each mean something slightly different
on each forge, and every one of those differences needs a test.

**The design is exact, so the code must be too.** A column one cell off is a
failing test (ADR-002). That is the point, and it makes every rendering
change slower than "looks about right" would.

**Agents and polling.** pullpen polls, because forge webhooks need a server
and pullpen has none. A sixty-second interval means an armed PR can merge a
minute before pullpen notices.
