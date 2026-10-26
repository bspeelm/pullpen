//! The forge-neutral query grammar (PLAN.md §6.1). Parsing and the terms
//! pullpen evaluates itself live here; turning the rest into each forge's
//! search is the forge crate's job.

use crate::pr::{Forge, Pr, Readiness};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Is {
    Open,
    Merged,
    Closed,
    Draft,
    Armed,
    Agent,
    Ready,
    Failing,
    Conflict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ci {
    Pass,
    Fail,
    Running,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Is(Is),
    /// A login, `@me`, or `@agents`.
    Author(String),
    ReviewRequested(String),
    ReviewedBy(String),
    /// `owner/name`, or a glob with `*`.
    Repo(String),
    Org(String),
    Group(String),
    Forge(Forge),
    Ci(Ci),
    Label(String),
    Base(String),
    /// `merged:7d`: merged within that long of now, as seconds.
    MergedWithin(i64),
    Text(String),
}

impl Term {
    /// Whether pullpen must evaluate this term on fetched PRs because no
    /// forge search understands it.
    pub fn client_side(&self) -> bool {
        match self {
            Term::Is(i) => matches!(i, Is::Armed | Is::Agent | Is::Ready | Is::Failing | Is::Conflict),
            Term::Author(a) => a == "@agents",
            Term::Repo(r) => r.contains('*'),
            Term::Ci(_) => true,
            _ => false,
        }
    }

    /// Whether this term alone keeps a search from meaning "every PR on the
    /// forge".
    pub fn narrows(&self) -> bool {
        matches!(
            self,
            Term::Author(_)
                | Term::ReviewRequested(_)
                | Term::ReviewedBy(_)
                | Term::Repo(_)
                | Term::Org(_)
                | Term::Group(_)
        ) && !self.client_side()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    pub terms: Vec<Term>,
}

impl Query {
    /// Unknown `key:value` terms are kept as free text, which matches titles:
    /// a typo narrows the list visibly instead of failing silently.
    pub fn parse(s: &str) -> Query {
        let terms = s
            .split_whitespace()
            .map(|tok| {
                let Some((k, v)) = tok.split_once(':') else { return Term::Text(tok.into()) };
                match (k, v) {
                    ("is", "open") => Term::Is(Is::Open),
                    ("is", "merged") => Term::Is(Is::Merged),
                    ("is", "closed") => Term::Is(Is::Closed),
                    ("is", "draft") => Term::Is(Is::Draft),
                    ("is", "armed") => Term::Is(Is::Armed),
                    ("is", "agent") => Term::Is(Is::Agent),
                    ("is", "ready") => Term::Is(Is::Ready),
                    ("is", "failing") => Term::Is(Is::Failing),
                    ("is", "conflict") => Term::Is(Is::Conflict),
                    ("author", a) => Term::Author(a.into()),
                    ("review-requested", a) => Term::ReviewRequested(a.into()),
                    ("reviewed-by", a) => Term::ReviewedBy(a.into()),
                    ("repo", r) => Term::Repo(r.into()),
                    ("org", o) => Term::Org(o.into()),
                    ("group", g) => Term::Group(g.into()),
                    ("forge", "gh") => Term::Forge(Forge::GitHub),
                    ("forge", "gl") => Term::Forge(Forge::GitLab),
                    ("ci", "pass") => Term::Ci(Ci::Pass),
                    ("ci", "fail") => Term::Ci(Ci::Fail),
                    ("ci", "running") => Term::Ci(Ci::Running),
                    ("label", l) => Term::Label(l.into()),
                    ("base", b) => Term::Base(b.into()),
                    ("merged", w) => crate::time::parse_age(w).map_or(Term::Text(tok.into()), Term::MergedWithin),
                    _ => Term::Text(tok.into()),
                }
            })
            .collect();
        Query { terms }
    }

    /// The forges this query reaches: those named by `forge:`, else all.
    pub fn reaches(&self, f: Forge) -> bool {
        let named: Vec<Forge> =
            self.terms.iter().filter_map(|t| if let Term::Forge(f) = t { Some(*f) } else { None }).collect();
        named.is_empty() || named.contains(&f)
    }

    pub fn narrowed(&self) -> bool {
        self.terms.iter().any(Term::narrows)
    }

    /// Evaluates the client-side terms; server-side ones are taken as
    /// already applied by the forge.
    pub fn keeps(&self, p: &Pr, r: Readiness) -> bool {
        self.terms.iter().filter(|t| t.client_side()).all(|t| match t {
            Term::Is(Is::Armed) => p.armed(),
            Term::Is(Is::Agent) | Term::Author(_) => p.author.is_agent,
            Term::Is(Is::Ready) => r == Readiness::Ready,
            Term::Is(Is::Failing) => r == Readiness::Failing,
            Term::Is(Is::Conflict) => r == Readiness::Conflict,
            Term::Repo(glob) => glob_match(glob, &p.id.repo),
            Term::Ci(Ci::Pass) => p.checks.all_passed(),
            Term::Ci(Ci::Fail) => p.checks.failing(),
            Term::Ci(Ci::Running) => p.checks.running > 0,
            _ => true,
        })
    }
}

/// `*` matches any run of characters; nothing else is special.
pub fn glob_match(glob: &str, s: &str) -> bool {
    let parts: Vec<&str> = glob.split('*').collect();
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if parts.len() == 1 {
        return glob == s;
    }
    if !s.starts_with(first) || !s[first.len()..].ends_with(last) {
        return false;
    }
    let mut rest = &s[first.len()..s.len() - last.len()];
    for mid in &parts[1..parts.len() - 1] {
        match rest.find(mid) {
            Some(i) => rest = &rest[i + mid.len()..],
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests;
