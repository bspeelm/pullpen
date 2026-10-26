//! Screens as lines of spans. Every function here is pure: state in, lines
//! out, no terminal. That is what lets the golden tests render them.

pub mod chrome;
pub mod help;
pub mod list;
pub mod preview;
pub mod words;

use std::collections::HashMap;

use pullpen_core::time::Timestamp;
use pullpen_core::{Capabilities, Pr, Readiness, readiness};

use crate::glyphs::Glyphs;

/// What every view needs besides its own state.
pub struct Ctx<'a> {
    pub now: Timestamp,
    pub glyphs: Glyphs,
    /// Per-repo merge rules, keyed `host/repo`; a repo not listed gets the
    /// permissive default until the forge has been asked.
    pub caps: &'a HashMap<String, Capabilities>,
}

impl Ctx<'_> {
    pub fn caps(&self, pr: &Pr) -> Capabilities {
        self.caps.get(&format!("{}/{}", pr.id.host, pr.id.repo)).cloned().unwrap_or_default()
    }
    pub fn readiness(&self, pr: &Pr) -> Readiness {
        readiness(pr, &self.caps(pr))
    }
}
