//! Same-person proposals (ADR 0019, 0029).
//!
//! Pattern: Strategy (client) — the repository applies the non-negotiable policy (no groups, no
//! sensitive cards, no already linked pairs) and delegates "who looks alike" to the injected
//! `MatchStrategy`.

use fs_objects::ObjectId;

use super::envelope::card_of;
use super::handle::PeopleRepo;
use super::types::ListFilter;
use crate::error::Result;
use crate::matching::{Candidate, SameAsProposal};

impl PeopleRepo {
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
            // One card that does not read blocks nobody: it is not proposed, `unreadable_cards` names it.
            let Ok(card) = card_of(&o) else { continue };
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

    /// The cards the store holds that do not read as a card, and why: left out of proposals, and
    /// the contract has no place for them yet (`DESIGN.md`).
    pub async fn unreadable_cards(&self) -> Result<Vec<(ObjectId, String)>> {
        let mut out = vec![];
        for o in self.list(&ListFilter::default()).await? {
            if let Err(e) = card_of(&o) {
                out.push((o.envelope.id, e.to_string()));
            }
        }
        Ok(out)
    }
}
