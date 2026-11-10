//! The commands of the person description that need the store: patch, check, suggest.
//!
//! Pattern: Command — each is one small operation over the pure functions in `person/`; the
//! repository only reads and writes.

use fs_objects::{ObjectId, Query, StoredObject};

use super::consts::{CONTEXT, PROFILE_TYPE};
use super::envelope::card_of;
use super::handle::PeopleRepo;
use crate::card::Card;
use crate::error::{Error, Issue, Result};
use crate::person::{self, PatchObject};

impl PeopleRepo {
    /// Change a card by a `PatchObject`; see [`crate::People::patch`]. `uid` and `@type` cannot be
    /// patched away: the result is validated and `update` refuses a changed `uid`.
    pub async fn patch(&self, id: ObjectId, patch: &PatchObject) -> Result<StoredObject> {
        let card = self.get_card(id).await?.ok_or(Error::NotFound(id))?;
        let mut value = card.to_value();
        patch.apply(&mut value)?;
        let changed: Card = serde_json::from_value(value).map_err(|_| {
            Error::Invalid(vec![Issue {
                path: "/".into(),
                message: "patch.not-a-card".into(),
            }])
        })?;
        self.update(id, changed).await
    }

    /// The problems of a stored card; see [`crate::People::check`].
    pub async fn check(&self, id: ObjectId) -> Result<Vec<Issue>> {
        let card = self.get_card(id).await?.ok_or(Error::NotFound(id))?;
        Ok(person::check(&card))
    }

    /// Completions for a field; see [`crate::People::suggest`]. The known values come from the first
    /// [`person::SCAN_LIMIT`] cards in name order. A sensitive card gives none (ADR 0029).
    pub async fn suggest(&self, field: &str, typed: &str) -> Result<Vec<String>> {
        if !person::is_suggestible(field) {
            return person::suggest(field, typed, std::iter::empty());
        }
        let mut q = Query::new()
            .of_type(PROFILE_TYPE)
            .in_context(CONTEXT)
            .order_by_index("name_sort");
        q.limit = Some(person::SCAN_LIMIT);
        let objects = self.reads.query(&q).await?;
        let cards: Vec<Card> = objects
            .iter()
            .filter(|o| o.envelope.sensitive.is_empty())
            .filter_map(|o| card_of(o).ok())
            .filter(|card| !card.is_sensitive())
            .collect();
        person::suggest(field, typed, &cards)
    }
}
