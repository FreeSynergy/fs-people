//! Card <-> stored object: envelope derivation and index fields.
//!
//! Pattern: Adapter — maps the JSContact card onto the AS2 `Profile` envelope + index fields that
//! `fs-store` understands (ADR 0020); the envelope `name` is derived, never typed (ADR 0032).

use fs_store::{Blob, Envelope, ObjectId, StoredObject};
use serde_json::Value;

use super::consts::{CONTENT_TYPE, CONTEXT, PROFILE_TYPE};
use super::handle::PeopleRepo;
use crate::card::Card;
use crate::error::{Error, MAX_CARD_BYTES, Result};
use crate::index::index_fields;

impl PeopleRepo {
    pub(super) fn build_object(
        &self,
        card: &Card,
        existing: Option<&Envelope>,
        original: Option<Blob>,
    ) -> StoredObject {
        let mut env = existing
            .cloned()
            .unwrap_or_else(|| self.fresh_envelope(card));
        env.name = card.display_name(); // ADR 0032: derived, never typed separately
        env.describes = card.describes().map(str::to_string); // AS2 Profile.describes
        env.sensitive = card.sensitive_categories(); // ADR 0029
        let mut o = StoredObject::new(env, CONTENT_TYPE, card.to_value());
        o.index = index_fields(card);
        o.original = original;
        o
    }

    fn fresh_envelope(&self, card: &Card) -> Envelope {
        let mut e = Envelope::new(PROFILE_TYPE, card.display_name());
        // A UUIDv7 uid doubles as object id, so a re-import maps onto the same object.
        if let Ok(id) = card.uid.parse::<ObjectId>() {
            e.id = id;
        }
        e.iri = Some(self.iri(e.id));
        e.context = Some(CONTEXT.into());
        e.attributed_to = Some(self.actor.clone());
        e.to = vec![self.actor.clone()];
        e
    }
}

/// Parse the content of a stored Profile.
pub fn card_of(o: &StoredObject) -> Result<Card> {
    stored_value(o.content.clone())
}

/// Read what the store holds; failing here is the fault of the data, not of the request.
pub(super) fn stored_value<T: serde::de::DeserializeOwned>(content: Value) -> Result<T> {
    serde_json::from_value(content).map_err(|e| Error::Stored(e.to_string()))
}

/// Refuse a card that could no longer come back in one answer.
pub(super) fn check_size(card: &Card) -> Result<()> {
    let bytes = card.to_value().to_string().len();
    if bytes > MAX_CARD_BYTES {
        return Err(Error::TooLarge(bytes));
    }
    Ok(())
}
