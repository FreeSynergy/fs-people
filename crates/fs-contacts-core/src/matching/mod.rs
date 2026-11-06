//! Same-person proposals (ADR 0019, 0029): module map.
//!
//! Pattern: Strategy (declarations) — [`MatchStrategy`] is the contract; `SharedContactPoint`
//! is the shipped implementation.

mod shared_point;
mod strategy;
mod types;

pub use shared_point::SharedContactPoint;
pub use strategy::MatchStrategy;
pub use types::{Candidate, MatchKind, MatchReason, SameAsProposal};
