//! Default strategy: two cards of the same kind that share an e-mail address or phone number.
//!
//! Pattern: Strategy (concrete) — groups candidates by shared value, then pairs them up.

use std::collections::{BTreeMap, BTreeSet};

use fs_store::ObjectId;

use super::strategy::MatchStrategy;
use super::types::{Candidate, MatchKind, MatchReason, SameAsProposal};

/// Shorter numbers (extensions, service codes) are not identifying.
const MIN_PHONE_DIGITS: usize = 6;

/// Shared normalised e-mail or phone number, same `kind`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SharedContactPoint;

impl MatchStrategy for SharedContactPoint {
    fn propose(&self, candidates: &[Candidate]) -> Vec<SameAsProposal> {
        let mut by_value: BTreeMap<(MatchKind, String), Vec<(ObjectId, &str)>> = BTreeMap::new();
        for c in candidates {
            let who = (c.id, c.kind.as_str());
            for e in &c.emails {
                by_value
                    .entry((MatchKind::Email, e.clone()))
                    .or_default()
                    .push(who);
            }
            for p in &c.phones {
                if p.trim_start_matches('+').len() >= MIN_PHONE_DIGITS {
                    by_value
                        .entry((MatchKind::Phone, p.clone()))
                        .or_default()
                        .push(who);
                }
            }
        }
        let mut pairs: BTreeMap<(ObjectId, ObjectId), BTreeSet<MatchReason>> = BTreeMap::new();
        for ((kind, value), ids) in by_value {
            for (i, (a, ka)) in ids.iter().enumerate() {
                for (b, kb) in &ids[i + 1..] {
                    if a == b || ka != kb {
                        continue;
                    }
                    let key = if a < b { (*a, *b) } else { (*b, *a) };
                    let reason = MatchReason {
                        kind,
                        value: value.clone(),
                    };
                    pairs.entry(key).or_default().insert(reason);
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
