//! The matcher that combines rules.
//!
//! Pattern: Strategy (concrete, composite) — groups candidates by the keys every rule gives them and
//! pairs those that share one. The default is [`RuleMatcher::standard`]: e-mail, phone, name at
//! organisation.

use std::collections::{BTreeMap, BTreeSet};

use fs_objects::ObjectId;

use super::rule::{MatchRule, SameNameAtOrganisation, SharedEmail, SharedPhone};
use super::strategy::MatchStrategy;
use super::types::{Candidate, MatchKind, MatchReason, SameAsProposal};

/// A matcher made of rules.
pub struct RuleMatcher {
    rules: Vec<Box<dyn MatchRule>>,
}

impl RuleMatcher {
    /// A matcher over these rules.
    #[must_use]
    pub fn new(rules: Vec<Box<dyn MatchRule>>) -> Self {
        Self { rules }
    }

    /// E-mail, phone and name at organisation.
    #[must_use]
    pub fn standard() -> Self {
        Self::new(vec![
            Box::new(SharedEmail),
            Box::new(SharedPhone),
            Box::new(SameNameAtOrganisation),
        ])
    }
}

impl MatchStrategy for RuleMatcher {
    fn propose(&self, candidates: &[Candidate]) -> Vec<SameAsProposal> {
        let mut by_key: BTreeMap<(MatchKind, String), Vec<(ObjectId, &str)>> = BTreeMap::new();
        for c in candidates {
            for rule in &self.rules {
                for key in rule.keys(c) {
                    by_key
                        .entry((rule.kind(), key))
                        .or_default()
                        .push((c.id, c.kind.as_str()));
                }
            }
        }
        let mut pairs: BTreeMap<(ObjectId, ObjectId), BTreeSet<MatchReason>> = BTreeMap::new();
        for ((kind, value), ids) in by_key {
            for (i, (a, ka)) in ids.iter().enumerate() {
                for (b, kb) in &ids[i + 1..] {
                    if a == b || ka != kb {
                        continue;
                    }
                    let key = if a < b { (*a, *b) } else { (*b, *a) };
                    pairs.entry(key).or_default().insert(MatchReason {
                        kind,
                        value: value.clone(),
                    });
                }
            }
        }
        pairs
            .into_iter()
            .map(|((a, b), r)| SameAsProposal {
                a,
                b,
                reasons: r.into_iter().collect(),
            })
            .collect()
    }
}
