//! `PeopleAdapter`: `dyn People` as a `PeopleSource`.
//!
//! Pattern: Adapter over the program's contract, driven with a tokio handle from the interface
//! thread. Provisional: in process, not RPC, until the Cap'n Proto servers exist (`DESIGN.md`).
//!
//! Revision: the store's `updated` time and a fingerprint of the content. `People::update` takes no
//! revision, so `save` compares and then writes; this is **not atomic** (another writer can slip in
//! between). A compare-and-set in the store is a wish for `fs-people-core`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use fs_components_workbenches::SourceError;
use fs_components_workbenches::people::{
    PeopleCapabilities, PeopleSource, PersonDraft, PersonRow, PersonSheet,
};
use fs_people_core::{Card, Error, ListFilter, People, RoleBook, card_of};
use fs_store::{ObjectId, StoredObject};
use tokio::runtime::Handle;

use crate::words::field;

use super::apply::apply_draft;
use super::draft::{blank_draft, draft_of};
use super::marks::is_marked;
use super::sheet::{first_email, sensitive_mark, sheet_of};

/// The contract of the people program, for the workbench. Do not call it from inside the runtime:
/// it blocks the calling thread until the core has answered.
pub struct PeopleAdapter {
    people: Arc<dyn People>,
    roles: Arc<dyn RoleBook>,
    runtime: Handle,
}

impl PeopleAdapter {
    /// An adapter over a provider of both contracts, driven with `runtime`.
    pub fn new(people: Arc<dyn People>, roles: Arc<dyn RoleBook>, runtime: Handle) -> Self {
        Self {
            people,
            roles,
            runtime,
        }
    }

    fn stored(&self, id: ObjectId) -> Result<StoredObject, SourceError> {
        self.runtime
            .block_on(self.people.get(id))
            .map_err(|e| reason(&e))?
            .ok_or_else(|| SourceError::not_found("This person no longer exists."))
    }
}

/// The key of a row: the object id.
fn id_of(key: &str) -> Result<ObjectId, SourceError> {
    key.parse()
        .map_err(|_| SourceError::not_found(format!("\"{key}\" names nobody.")))
}

/// What the core said, in the workbench's terms. A store refusing a sensitive card is said plainly.
fn reason(error: &Error) -> SourceError {
    match error {
        Error::Store(fs_store::Error::EncryptionRequired(_)) => SourceError::failed(format!(
            "this card is marked sensitive and this store is not encrypted, so it was refused ({error})"
        )),
        Error::NotFound(_) => SourceError::not_found(error.to_string()),
        Error::Unsupported(_) => SourceError::unsupported(error.to_string()),
        _ => SourceError::failed(error.to_string()),
    }
}

/// The same reason as plain text.
pub(crate) fn reason_text(error: &Error) -> String {
    reason(error).message
}

/// `updated` alone could repeat within the store's clock tick.
fn revision_of(object: &StoredObject) -> String {
    let mut hash = DefaultHasher::new();
    object.content.to_string().hash(&mut hash);
    format!(
        "{}/{:x}",
        object.envelope.updated.to_rfc3339(),
        hash.finish()
    )
}

impl PeopleSource for PeopleAdapter {
    fn capabilities(&self) -> PeopleCapabilities {
        let report = self.people.capabilities();
        let mut can = PeopleCapabilities::read_only();
        can.edit = report.is_supported("person.update");
        can.create = report.is_supported("person.create");
        can.delete = report.is_supported("person.delete");
        can
    }

    fn rows(&self, term: &str, offset: usize, limit: usize) -> Result<Vec<PersonRow>, SourceError> {
        let found = if term.trim().is_empty() {
            let filter = ListFilter {
                limit: Some(limit),
                offset,
                ..ListFilter::default()
            };
            self.runtime.block_on(self.people.list(&filter))
        } else {
            // `People::search` has no paging: the whole result is read for every page (DESIGN.md).
            self.runtime
                .block_on(self.people.search(term))
                .map(|all| all.into_iter().skip(offset).take(limit).collect())
        };
        Ok(found
            .map_err(|e| reason(&e))?
            .iter()
            .map(|object| {
                // A card that cannot be parsed is still listed under its name, so it can be found.
                let mail = card_of(object)
                    .ok()
                    .as_ref()
                    .map_or_else(String::new, first_email);
                let row = PersonRow::new(
                    object.envelope.id.to_string(),
                    vec![object.envelope.name.clone(), mail],
                );
                if is_marked(object) {
                    row.guarded(field::SENSITIVE_ROW)
                } else {
                    row
                }
            })
            .collect())
    }

    fn read(&self, key: &str) -> Result<PersonSheet, SourceError> {
        let id = id_of(key)?;
        let object = self.stored(id)?;
        let card = card_of(&object).map_err(|e| reason(&e))?;
        Ok(sheet_of(&card, &object, self.roles.as_ref(), &self.runtime))
    }

    fn draft(&self, key: &str) -> Result<PersonDraft, SourceError> {
        let object = self.stored(id_of(key)?)?;
        let card = card_of(&object).map_err(|e| reason(&e))?;
        let mut draft = draft_of(&card, Some(revision_of(&object)));
        draft.marks.extend(sensitive_mark(&card, &object));
        Ok(draft)
    }

    fn blank(&self) -> Result<PersonDraft, SourceError> {
        Ok(blank_draft())
    }

    /// The form goes onto the card as it is stored now, so whatever the form does not show stays.
    fn save(&self, key: &str, draft: &PersonDraft) -> Result<(), SourceError> {
        let id = id_of(key)?;
        let object = self.stored(id)?;
        if draft.revision.as_deref() != Some(revision_of(&object).as_str()) {
            return Err(SourceError::conflict(
                "Someone else changed this person since you started.",
            ));
        }
        let mut card = card_of(&object).map_err(|e| reason(&e))?;
        apply_draft(&mut card, draft)?;
        self.runtime
            .block_on(self.people.update(id, card))
            .map(|_| ())
            .map_err(|e| reason(&e))
    }

    fn create(&self, draft: &PersonDraft) -> Result<String, SourceError> {
        let mut card = Card::individual("");
        card.name = None;
        apply_draft(&mut card, draft)?;
        self.runtime
            .block_on(self.people.create(card))
            .map(|object| object.envelope.id.to_string())
            .map_err(|e| reason(&e))
    }

    fn delete(&self, key: &str) -> Result<(), SourceError> {
        self.runtime
            .block_on(self.people.delete(id_of(key)?))
            .map_err(|e| reason(&e))
    }
}
