//! Collector for validation problems.
//!
//! Pattern: Collecting parameter — every rule reports into one `Sink`, so a card is checked
//! completely and the caller sees all problems at once.

use crate::error::Issue;

/// Collects [`Issue`]s.
#[derive(Default)]
pub(super) struct Sink(pub(super) Vec<Issue>);

impl Sink {
    /// Record one problem at `path`.
    pub(super) fn bad(&mut self, path: &str, msg: &str) {
        self.0.push(Issue {
            path: path.into(),
            message: msg.into(),
        });
    }
}
