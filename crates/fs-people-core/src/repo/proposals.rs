//! Same-person proposals (ADR 0019, 0029).
//!
//! Pattern: Strategy (client) — the repository applies the non-negotiable policy (no groups, no
//! sensitive cards, no already linked pairs) and delegates "who looks alike" to the injected
//! `MatchStrategy`.

use super::envelope::card_of;
use super::handle::ContactRepo;
use super::types::ListFilter;
use crate::error::Result;
use crate::matching::{Candidate, SameAsProposal};

impl ContactRepo {
    /// Candidate pairs the match strategy proposes (by default: a shared normalised e-mail
    /// address or phone number).
    ///
    /// Never proposed: cards with sensitive categories (ADR 0029), groups, pairs of different
    /// `kind`, pairs that are already linked. Pure function of the store's state; nothing is
    /// written.
    pub async fn find_same_person_proposals(&self) -> Result<Vec<SameAsProposal>> {
        let linked = self.linked_pairs().await?;
        let mut candidates = vec![];
        for o in self.list(&ListFilter::default()).await? {
            let card = card_of(&o)?;
            if card.is_group() || card.is_sensitive() || !o.envelope.sensitive.is_empty() {
                continue;
            }
            let values = |key: &str| o.index.get(key).cloned().unwrap_or_default();
            candidates.push(Candidate {
                id: o.envelope.id,
                kind: card.kind().to_string(),
                emails: values("email"),
                phones: values("phone"),
            });
        }
        let mut proposals = self.matcher.propose(&candidates);
        proposals.retain(|p| !linked.contains(&(p.a, p.b)));
        Ok(proposals)
    }
}
