//! Sections: named queries, shown as tabs.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    Updated,
    Created,
    Age,
    Repo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    List,
    /// The grouped lifecycle view (design §06).
    Armed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub short: String,
    pub query: String,
    pub sort: Sort,
    pub view: View,
    /// Whether this section's PRs feed the status line's armed and failing
    /// counts. Off for a catch-all section, whose failures are mostly other
    /// people's and not yours to act on.
    pub counts: bool,
}
