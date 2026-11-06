//! Relations of a person, stored on the card.
//!
//! Pattern: Command (client) — the small edits of `card/relations.rs` applied through `update`.

use fs_store::{ObjectId, StoredObject};

use super::handle::PeopleRepo;
use crate::error::{Error, Result};

impl PeopleRepo {
    /// The card with `other_uid` is `relation` to the card of `id` (RFC 9553 §2.1.8).
    pub async fn relate(
        &self,
        id: ObjectId,
        other_uid: &str,
        relation: &str,
    ) -> Result<StoredObject> {
        if other_uid.trim().is_empty() || relation.trim().is_empty() {
            return Err(Error::Conflict("a relation needs a uid and a type".into()));
        }
        let mut card = self.get_card(id).await?.ok_or(Error::NotFound(id))?;
        card.relate(other_uid, relation);
        self.update(id, card).await
    }

    /// Remove one relation.
    pub async fn unrelate(
        &self,
        id: ObjectId,
        other_uid: &str,
        relation: &str,
    ) -> Result<StoredObject> {
        let mut card = self.get_card(id).await?.ok_or(Error::NotFound(id))?;
        card.unrelate(other_uid, relation);
        self.update(id, card).await
    }

    /// The relations of a card as `(uid, relation)` pairs.
    pub async fn relations(&self, id: ObjectId) -> Result<Vec<(String, String)>> {
        let card = self.get_card(id).await?.ok_or(Error::NotFound(id))?;
        Ok(card.relations())
    }
}
