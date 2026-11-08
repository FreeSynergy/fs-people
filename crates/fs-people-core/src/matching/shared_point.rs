//! The earlier default strategy: two cards of the same kind that share an e-mail address or phone number.
//!
//! Pattern: Strategy (concrete) — now the two rules `SharedEmail` and `SharedPhone` in a
//! [`RuleMatcher`]; kept under its name so a host that chose it keeps it.

use super::rule::{SharedEmail, SharedPhone};
use super::rules::RuleMatcher;
use super::strategy::MatchStrategy;
use super::types::{Candidate, SameAsProposal};

/// Shared normalised e-mail or phone number, same `kind`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SharedContactPoint;

impl MatchStrategy for SharedContactPoint {
    fn propose(&self, candidates: &[Candidate]) -> Vec<SameAsProposal> {
        RuleMatcher::new(vec![Box::new(SharedEmail), Box::new(SharedPhone)]).propose(candidates)
    }
}
