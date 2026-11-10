//! Create, read, update and delete of single cards.
//!
//! Pattern: Repository — the write side of the people program; every change goes through the
//! store, which logs it and makes it undoable.

use fs_objects::{Blob, Cond, ObjectId, Op, Query, StoredObject};

use super::consts::PROFILE_TYPE;
use super::envelope::{card_of, check_size};
use super::handle::PeopleRepo;
use crate::card::Card;
use crate::clock::now_utc;
use crate::error::{Error, Issue, Result};
use crate::feed::PersonChange;
use crate::index::normalise_uid;
use crate::validate::{has_negative_zero, inexact, validate};

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
        card.rewrite_legacy();
        card.created.get_or_insert_with(|| now.clone());
        card.updated.get_or_insert(now);
        validate(&card)?;
        check_size(&card)?;
        if has_negative_zero(&card.to_value()) {
            return Err(inexact("-0"));
        }
        if let Some(existing) = self.get_by_uid(&card.uid).await? {
            return Err(Error::Exists(existing.envelope.id));
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
        // What a delete that stopped halfway left behind must not pass to a new card of that id.
        let uid_is_id = normalise_uid(&card.uid).parse::<ObjectId>().is_ok();
        if uid_is_id
            && (!self.relationships_of(id).await?.is_empty()
                || !self.roles_of(id).await?.is_empty()
                || !self.assignments_scoped_to(&self.iri(id)).await?.is_empty())
        {
            return Err(Error::Conflict(format!(
                "links or roles of an earlier {id}, or held within it, remain; delete it again first"
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

    /// Replace the card of `id`. The `uid` cannot change; `updated` is set to now. An edit never takes
    /// a sensitive marking away (only an unclear one can be replaced by named categories), and a
    /// card that is now sensitive loses its same-person links.
    pub async fn update(&self, id: ObjectId, mut card: Card) -> Result<StoredObject> {
        let old = self.get(id).await?.ok_or(Error::NotFound(id))?;
        let old_card = card_of(&old)?;
        if old_card.uid != card.uid {
            return Err(Error::Conflict("the uid of a card cannot change".into()));
        }
        if card.kind() != old_card.kind() {
            self.check_kind_against_roles(id, card.kind()).await?;
        }
        card.rewrite_legacy();
        card.keep_marks_of(&old_card);
        card.created = card.created.or(old_card.created);
        card.updated = Some(now_utc());
        validate(&card)?;
        check_size(&card)?;
        // One an older card holds stays possible to edit around; a new one does not come in.
        if has_negative_zero(&card.to_value()) && !has_negative_zero(&old.content) {
            return Err(inexact("-0"));
        }
        let obj = self.build_object(&card, Some(&old.envelope), None);
        self.objects.put(obj, &self.actor).await?;
        let stored = self.get(id).await?.ok_or(Error::NotFound(id))?;
        self.feed.notify(&PersonChange::Updated(id));
        // ADR 0029: a sensitive card is never linked, also not by a link older than its marking.
        if card.is_sensitive() {
            for link in self.relationships_of(id).await? {
                self.objects.delete(link.envelope.id, &self.actor).await?;
            }
            for rejection in self.rejections_of(id).await? {
                self.objects
                    .delete(rejection.envelope.id, &self.actor)
                    .await?;
            }
        }
        Ok(stored)
    }
}
