//! Pairs a person said "not the same person" to (ADR 0058).
//!
//! Pattern: Repository + State — a rejection is an AS2 `Reject` object with the pair in `subject` /
//! `object` index fields (smaller id first), so it is logged, undoable and queryable like a link.
//! `pair_state` reads link and rejection into a [`ProposalState`]; `reject`, `reopen` and the steps of
//! `links.rs` go through `ProposalState::on`, so no path allows a step the table does not.

use std::collections::BTreeSet;

use fs_objects::{Envelope, ObjectId, Query, StoredObject};
use serde_json::json;

use super::consts::{AS2_TYPE, CONTEXT, REJECTION_TYPE, SAME_AS};
use super::handle::PeopleRepo;
use crate::error::{Error, Result};
use crate::matching::{ProposalState, Step};

pub(super) fn ordered(a: ObjectId, b: ObjectId) -> (ObjectId, ObjectId) {
    if a < b { (a, b) } else { (b, a) }
}

impl PeopleRepo {
    /// Where the pair stands: `Confirmed` if linked, `Rejected` if a person said no, otherwise
    /// `None` (nobody has answered; it is `Proposed` only if the rules find it).
    ///
    /// # Errors
    /// The store fails.
    pub async fn same_person_state(
        &self,
        a: ObjectId,
        b: ObjectId,
    ) -> Result<Option<ProposalState>> {
        let key = ordered(a, b);
        if self.linked_pairs().await?.contains(&key) {
            return Ok(Some(ProposalState::Confirmed));
        }
        Ok(self
            .rejection_of(key)
            .await?
            .map(|_| ProposalState::Rejected))
    }

    /// A person says these two cards are not the same person: the pair is not proposed again.
    /// Refused for a pair that is linked (unlink it first) or already rejected, and for a card that
    /// does not exist.
    ///
    /// # Errors
    /// A card is missing, the pair is not `Proposed`, or the store fails.
    pub async fn reject_same_person(&self, a: ObjectId, b: ObjectId) -> Result<StoredObject> {
        if a == b {
            return Err(Error::Conflict("a card is the same as itself".into()));
        }
        let (oa, ob) = (
            self.get(a).await?.ok_or(Error::NotFound(a))?,
            self.get(b).await?.ok_or(Error::NotFound(b))?,
        );
        let key = ordered(a, b);
        // A pair nobody answered is `Proposed`; a linked or already rejected one cannot be rejected.
        ProposalState::on(
            self.same_person_state(a, b)
                .await?
                .unwrap_or(ProposalState::Proposed),
            Step::Reject,
        )?;
        let mut env = Envelope::new(
            REJECTION_TYPE,
            format!(
                "{} != {} (not the same person)",
                oa.envelope.name, ob.envelope.name
            ),
        );
        env.context = Some(CONTEXT.into());
        env.attributed_to = Some(self.actor.clone());
        env.to = vec![self.actor.clone()];
        env.iri = Some(self.iri(env.id));
        let content = json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": REJECTION_TYPE,
            "actor": self.actor,
            "object": {"type": "Relationship", "subject": self.iri(key.0), "relationship": SAME_AS, "object": self.iri(key.1)},
        });
        let obj = StoredObject::new(env, AS2_TYPE, content)
            .with_index("relationship", SAME_AS)
            .with_index("subject", key.0.to_string())
            .with_index("object", key.1.to_string());
        let id = obj.envelope.id;
        self.objects.put(obj, &self.actor).await?;
        self.objects.get(id).await?.ok_or(Error::NotFound(id))
    }

    /// Take a rejection back: the pair may be proposed again.
    ///
    /// # Errors
    /// The id is no rejection, or the store fails.
    pub async fn reopen_same_person(&self, rejection: ObjectId) -> Result<()> {
        let found = self.objects.get(rejection).await?;
        if found
            .as_ref()
            .is_none_or(|o| o.envelope.object_type != REJECTION_TYPE)
        {
            return Err(fs_objects::Error::NotFound(rejection).into());
        }
        ProposalState::Rejected.on(Step::Reopen)?;
        self.objects.delete(rejection, &self.actor).await?;
        Ok(())
    }

    /// The pairs a person rejected, smaller id first.
    pub(super) async fn rejected_pairs(&self) -> Result<BTreeSet<(ObjectId, ObjectId)>> {
        Ok(self
            .rejections()
            .await?
            .iter()
            .filter_map(pair_of)
            .collect())
    }

    /// The rejection object of a pair, if any.
    pub(super) async fn rejection_of(
        &self,
        key: (ObjectId, ObjectId),
    ) -> Result<Option<StoredObject>> {
        Ok(self
            .rejections()
            .await?
            .into_iter()
            .find(|r| pair_of(r) == Some(key)))
    }

    /// Rejections that mention a card (deleted with the card).
    pub(super) async fn rejections_of(&self, id: ObjectId) -> Result<Vec<StoredObject>> {
        Ok(self
            .rejections()
            .await?
            .into_iter()
            .filter(|r| pair_of(r).is_some_and(|(x, y)| x == id || y == id))
            .collect())
    }

    async fn rejections(&self) -> Result<Vec<StoredObject>> {
        let q = Query::new().of_type(REJECTION_TYPE).in_context(CONTEXT);
        Ok(self.reads.query(&q).await?)
    }
}

fn pair_of(r: &StoredObject) -> Option<(ObjectId, ObjectId)> {
    let get = |k: &str| r.index.get(k)?.first()?.parse().ok();
    Some((get("subject")?, get("object")?))
}
