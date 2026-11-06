//! Create, read, update and delete of single cards.
//!
//! Pattern: Repository — the write side of the people program; every change goes through the
//! store, which logs it and makes it undoable.

use fs_store::{Blob, Cond, ObjectId, Op, Query, StoredObject};

use super::consts::PROFILE_TYPE;
use super::envelope::{card_of, check_size};
use super::handle::PeopleRepo;
use crate::card::Card;
use crate::clock::now_utc;
use crate::error::{Error, Issue, Result};
use crate::feed::PersonChange;
use crate::index::normalise_uid;
use crate::validate::validate;

impl PeopleRepo {
    /// Validate and store a new card. Refused if its `uid` already exists, in any spelling, and if
    /// it carries a sensitive marking that names no category: whoever types it can say what is
    /// meant (an import keeps such a card, see `import`).
    pub async fn create(&self, card: Card) -> Result<StoredObject> {
        let unclear = card.unclear_sensitive_paths();
        if !unclear.is_empty() {
            return Err(Error::Invalid(
                unclear
                    .into_iter()
                    .map(|path| Issue {
                        path,
                        message: "a sensitive marking must name its categories".into(),
                    })
                    .collect(),
            ));
        }
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
        check_size(&card)?;
        if self.get_by_uid(&card.uid).await?.is_some() {
            return Err(Error::Conflict(format!(
                "a person with uid {} exists",
                card.uid
            )));
        }
        let obj = self.build_object(&card, None, original);
        let id = obj.envelope.id;
        // A UUIDv7 uid doubles as object id; `put` would silently overwrite what already has it.
        if self.objects.get(id).await?.is_some() {
            return Err(Error::Conflict(format!(
                "the uid {} is the id of another object",
                card.uid
            )));
        }
        self.objects.put(obj, &self.actor).await?;
        let stored = self.get(id).await?.ok_or(Error::NotFound(id))?;
        self.feed.notify(&PersonChange::Created(id));
        Ok(stored)
    }

    /// The stored object of a person, if it exists.
    pub async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        Ok(self
            .objects
            .get(id)
            .await?
            .filter(|o| o.envelope.object_type == PROFILE_TYPE))
    }

    /// The card of a person, if it exists.
    pub async fn get_card(&self, id: ObjectId) -> Result<Option<Card>> {
        self.get(id).await?.map(|o| card_of(&o)).transpose()
    }

    /// Lookup by the card's `uid`, in any spelling of the same identity. Only the `uid` counts: a card
    /// whose object id happens to equal `uid`, with another `uid` of its own, is not found here, though
    /// `create` still refuses to overwrite it. Cards stored before the index `uid_key` existed are
    /// found by a search over `uid`, whatever their case.
    pub async fn get_by_uid(&self, uid: &str) -> Result<Option<StoredObject>> {
        let key = normalise_uid(uid);
        let same = |o: &StoredObject| {
            o.content
                .get("uid")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|stored| normalise_uid(stored) == key)
        };
        if let Ok(id) = key.parse::<ObjectId>()
            && let Some(found) = self.get(id).await?.filter(same)
        {
            return Ok(Some(found));
        }
        let is = |index: &str, op: Op, value: &str| Cond::Index {
            key: index.into(),
            op,
            value: value.into(),
        };
        let mut q = Query::new()
            .of_type(PROFILE_TYPE)
            .or(is("uid_key", Op::Eq, &key))
            .or(is("uid", Op::Eq, uid));
        if key.len() == 36 && key.as_bytes()[8] == b'-' {
            // Older cards: the same UUID in another case, with or without hyphens or `urn:uuid:`.
            q = q.or(is("uid", Op::Contains, &key)).or(is(
                "uid",
                Op::Contains,
                &key.replace('-', ""),
            ));
        }
        Ok(self.reads.query(&q).await?.into_iter().find(same))
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
        check_size(&card)?;
        let obj = self.build_object(&card, Some(&old.envelope), None);
        self.objects.put(obj, &self.actor).await?;
        let stored = self.get(id).await?.ok_or(Error::NotFound(id))?;
        self.feed.notify(&PersonChange::Updated(id));
        Ok(stored)
    }

    /// Delete a card (undoable through the store). Its role assignments and the same-person links
    /// that mention it are removed too. The store has no transaction over several objects, so the
    /// card goes first: if that fails nothing has changed. If removing the rest then fails, the
    /// error says so and the same call again removes what is left.
    pub async fn delete(&self, id: ObjectId) -> Result<()> {
        let card_there = self.get(id).await?.is_some();
        if card_there {
            self.objects.delete(id, &self.actor).await?;
            self.feed.notify(&PersonChange::Deleted(id));
        }
        let cleaned = self.remove_what_belongs_to(id, card_there).await;
        match cleaned {
            Err(e) if card_there && !matches!(e, Error::NotFound(_)) => {
                Err(Error::PartlyDeleted(id, e.to_string()))
            }
            other => other,
        }
    }

    /// The links and roles of `id`. `NotFound` if there was neither a card nor anything of this.
    async fn remove_what_belongs_to(&self, id: ObjectId, card_was_there: bool) -> Result<()> {
        let links = self.relationships_of(id).await?;
        let roles = self.roles_of(id).await?;
        if !card_was_there && links.is_empty() && roles.is_empty() {
            return Err(Error::NotFound(id));
        }
        for link in links {
            self.objects.delete(link.envelope.id, &self.actor).await?;
        }
        self.delete_assignments_of(id).await
    }
}
