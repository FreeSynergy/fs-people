//! Data types of the same-person matching.
//!
//! Pattern: Value objects — plain data passed between the repository and a [`MatchStrategy`].
//!
//! [`MatchStrategy`]: super::MatchStrategy

use fs_objects::ObjectId;

/// What a proposal is based on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchKind {
    /// A shared normalised e-mail address.
    Email,
    /// A shared normalised phone number.
    Phone,
}

/// One reason for a proposal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MatchReason {
    /// What matched.
    pub kind: MatchKind,
    /// The normalised value both cards share.
    pub value: String,
}

/// A suggested `owl:sameAs` link between two cards. Nothing is linked until it is confirmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameAsProposal {
    /// `a < b` (ids are time-ordered), so a pair appears once.
    pub a: ObjectId,
    /// The second card of the pair.
    pub b: ObjectId,
    /// Why the cards are suggested.
    pub reasons: Vec<MatchReason>,
}

/// What a strategy may look at: the identifying facts of one eligible card. Groups and sensitive
/// cards never become candidates (policy of the repository, not of the strategy).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The card's object id.
    pub id: ObjectId,
    /// JSContact `kind`; cards of different kind are never the same person.
    pub kind: String,
    /// Normalised e-mail addresses.
    pub emails: Vec<String>,
    /// Normalised phone numbers.
    pub phones: Vec<String>,
}
