//! The domain: PRs, readiness, sections, queries, actions and events.
//!
//! No I/O and no rendering live here, so both the forge clients and the UI
//! can depend on it without depending on each other (PLAN.md §2–§3).

pub mod action;
pub mod detail;
pub mod pr;
pub mod query;
pub mod section;
pub mod time;
pub mod verdict;

pub use action::*;
pub use detail::*;
pub use pr::*;
pub use section::*;
pub use verdict::*;
