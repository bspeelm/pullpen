# Security

pullpen acts on your behalf against GitHub and GitLab, using a token it
borrows and never stores (ADR-006). A flaw that leaks that token, or makes
pullpen merge, close or approve something you did not confirm, is a security
issue.

Report it privately through GitHub's
[security advisory form](https://github.com/lampseen/pullpen/security/advisories/new),
not the public tracker. You will get an answer within a week.
