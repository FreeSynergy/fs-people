//! Same-person proposals (ADR 0019, 0029, 0058): module map.
//!
//! Pattern: Strategy (declarations) — [`MatchStrategy`] is the contract; [`RuleMatcher`] combines
//! [`MatchRule`]s (Specification, one per feature); [`ProposalState`] is where a pair stands (State).

mod rule;
mod rules;
mod shared_point;
mod state;
mod strategy;
mod types;

pub use rule::{MatchRule, SameNameAtOrganisation, SharedEmail, SharedPhone};
pub use rules::RuleMatcher;
pub use shared_point::SharedContactPoint;
pub use state::{ProposalState, Step};
pub use strategy::MatchStrategy;
pub use types::{Candidate, MatchKind, MatchReason, SameAsProposal};
