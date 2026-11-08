//! One reason two cards may be the same person, as a rule of its own.
//!
//! Pattern: Specification — a [`MatchRule`] says by which keys a candidate satisfies one feature
//! (shared e-mail address, shared phone number, same name at the same organisation); two candidates
//! "fit together" under the rule when they share a key. Rules are pure, testable alone, and
//! combined by [`RuleMatcher`](super::RuleMatcher); a new feature is a new rule, not an edit of the
//! matcher (ADR 0058: rules per feature, normalised values).

use super::types::{Candidate, MatchKind};

/// Shorter numbers (extensions, service codes) are not identifying.
const MIN_PHONE_DIGITS: usize = 6;

/// A feature by which cards can fit together.
pub trait MatchRule: Send + Sync {
    /// What a match under this rule is called.
    fn kind(&self) -> MatchKind;
    /// The keys of `candidate` under this rule; two candidates fit if they share a key. Values are
    /// already normalised; an empty list means the candidate has nothing to offer here.
    fn keys(&self, candidate: &Candidate) -> Vec<String>;
}

/// A shared e-mail address.
#[derive(Clone, Copy, Debug, Default)]
pub struct SharedEmail;

impl MatchRule for SharedEmail {
    fn kind(&self) -> MatchKind {
        MatchKind::Email
    }
    fn keys(&self, c: &Candidate) -> Vec<String> {
        c.emails.clone()
    }
}

/// A shared phone number of at least six digits.
#[derive(Clone, Copy, Debug, Default)]
pub struct SharedPhone;

impl MatchRule for SharedPhone {
    fn kind(&self) -> MatchKind {
        MatchKind::Phone
    }
    fn keys(&self, c: &Candidate) -> Vec<String> {
        c.phones
            .iter()
            .filter(|p| p.trim_start_matches('+').len() >= MIN_PHONE_DIGITS)
            .cloned()
            .collect()
    }
}

/// The same name at the same organisation. Both must be there: a name alone is no key.
#[derive(Clone, Copy, Debug, Default)]
pub struct SameNameAtOrganisation;

impl MatchRule for SameNameAtOrganisation {
    fn kind(&self) -> MatchKind {
        MatchKind::NameAndOrganisation
    }
    fn keys(&self, c: &Candidate) -> Vec<String> {
        if c.name.is_empty() {
            return vec![];
        }
        c.orgs
            .iter()
            .filter(|o| !o.is_empty())
            .map(|o| format!("{} @ {o}", c.name))
            .collect()
    }
}
