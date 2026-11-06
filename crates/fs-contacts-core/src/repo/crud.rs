//! Create, read, update and delete of single cards.
//!
//! Pattern: Repository — the write side of the contact book; every change goes through the
//! store, which logs it and makes it undoable.

use fs_store::{Blob, ObjectId, Query, StoredObject};

use super::consts::PROFILE_TYPE;
use super::envelope::card_of;
use super::handle::ContactRepo;
use crate::card::Card;
use crate::clock::now_utc;
use crate::error::{Error, Result};
use crate::validate::validate;

impl ContactRepo {
    /// Validate and store a new card. Refused if its `uid` already exists.
    pub async fn create(&self, card: Card) -> Result<StoredObject> {
        self.create_inner(card, None).await
    }

    pub(super) async fn create_inner(
        &self,
        mut card: Card,
        original: Option<Blob>,
    ) -> Result<StoredObject> {
        let now = now_utc();
        card.created.get_or_insert_with(|| now.clone());
        card.updated.get_or_insert(now);
        validate(&card)?;
        if self.get_by_uid(&card.uid).await?.is_some() {
            return Err(Error::Conflict(format!(
                "a contact with uid {} exists",
                card.uid
            )));
        }
        let obj = self.build_object(&card, None, original);
        let id = obj.envelope.id;
        self.objects.put(obj, &self.actor).await?;
        self.get(id).await?.ok_or(Error::NotFound(id))
    }

    /// The stored object of a contact, if it exists.
    pub async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        Ok(self
            .objects
            .get(id)
            .await?
            .filter(|o| o.envelope.object_type == PROFILE_TYPE))
    }

    /// The card of a contact, if it exists.
    pub async fn get_card(&self, id: ObjectId) -> Result<Option<Card>> {
        self.get(id).await?.map(|o| card_of(&o)).transpose()
    }

    /// Lookup by the card's `uid`.
    pub async fn get_by_uid(&self, uid: &str) -> Result<Option<StoredObject>> {
        let q = Query::new()
            .of_type(PROFILE_TYPE)
            .with_index("uid", uid)
            .limit(1);
        Ok(self.reads.query(&q).await?.into_iter().next())
    }

    /// Replace the card of `id`. The `uid` cannot change; `updated` is set to now.
    pub async fn update(&self, id: ObjectId, mut card: Card) -> Result<StoredObject> {
        let old = self.get(id).await?.ok_or(Error::NotFound(id))?;
        let old_card = card_of(&old)?;
        if old_card.uid != card.uid {
            return Err(Error::Conflict("the uid of a card cannot change".into()));
        }
        card.created = card.created.or(old_card.created);
        card.updated = Some(now_utc());
        validate(&card)?;
        let obj = self.build_object(&card, Some(&old.envelope), None);
        self.objects.put(obj, &self.actor).await?;
        self.get(id).await?.ok_or(Error::NotFound(id))
    }

    /// Delete a card (undoable through the store). Same-person links that mention it are removed
    /// too.
    pub async fn delete(&self, id: ObjectId) -> Result<()> {
        self.get(id).await?.ok_or(Error::NotFound(id))?;
        for rel in self.relationships_of(id).await? {
            self.objects.delete(rel.envelope.id, &self.actor).await?;
        }
        self.objects.delete(id, &self.actor).await?;
        Ok(())
    }
}
