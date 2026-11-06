//! The [`MatchStrategy`] contract.
//!
//! Pattern: Strategy — how "probably the same person" is decided is swappable (shared contact
//! point today; fuzzy names or a trained model later) without touching the repository.

use super::types::{Candidate, SameAsProposal};

/// Finds pairs of candidates that are probably the same person.
pub trait MatchStrategy: Send + Sync {
    /// Proposals among `candidates`: each pair once with `a < b`, reasons sorted. Pure: nothing
    /// is read or written outside the arguments.
    fn propose(&self, candidates: &[Candidate]) -> Vec<SameAsProposal>;
}
