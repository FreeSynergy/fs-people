//! `PeopleAdapter`: `dyn People` as a `PeopleSource`.
//!
//! Pattern: Adapter over the program's contract (`People` and `RoleBook`, in practice the client of
//! `people.capnp`), driven with a tokio handle from the interface thread: every call blocks that
//! thread until the server has answered (`DESIGN.md`).
//!
//! Revision: the store's `updated` time and a fingerprint of the content. `People::update` takes no
//! revision, so `save` compares and then writes; this is **not atomic** (another writer can slip in
//! between). A compare-and-set in the store is a wish for `fs-people-core`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, PoisonError};

use fs_components_workbenches::SourceError;
use fs_components_workbenches::people::{
    PeopleCapabilities, PeopleSource, PersonDraft, PersonRow, PersonSheet,
};
use fs_people_client::PersonPages;
use fs_people_core::{Card, Error, People, RoleBook, card_of};
use fs_store::{ObjectId, StoredObject};
use tokio::runtime::Handle;

use super::apply::apply_draft;
use super::draft::{blank_draft, draft_of};
use super::reading::Reading;
use super::sheet::{row_of, sensitive_mark, sheet_of};

/// The contract of the people program, for the workbench. Do not call it from inside the runtime:
/// it blocks the calling thread until the provider has answered.
pub struct PeopleAdapter {
    people: Arc<dyn People>,
    roles: Arc<dyn RoleBook>,
    pages: Arc<dyn PersonPages>,
    reading: Mutex<Option<Reading>>,
    runtime: Handle,
}

impl PeopleAdapter {
    /// An adapter over a provider of both contracts and of the page-wise reading, driven with
    /// `runtime`.
    pub fn new(
        people: Arc<dyn People>,
        roles: Arc<dyn RoleBook>,
        pages: Arc<dyn PersonPages>,
        runtime: Handle,
    ) -> Self {
        Self {
            people,
            roles,
            pages,
            reading: Mutex::new(None),
            runtime,
        }
    }

    /// The rows from `offset`, read on from the cursor of the last call when it ended there. A reading
    /// that failed is dropped, so the next call starts clean.
    fn read_rows(
        &self,
        term: &str,
        offset: usize,
        limit: usize,
    ) -> fs_people_core::Result<Vec<StoredObject>> {
        let mut slot = self.reading.lock().unwrap_or_else(PoisonError::into_inner);
        let mut reading = match slot.take() {
            Some(r) if r.term == term => r,
            _ => Reading::new(self.pages.clone(), term),
        };
        let rows = self.runtime.block_on(reading.take(offset, limit))?;
        *slot = Some(reading);
        Ok(rows)
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
        let found = self.read_rows(term, offset, limit);
        Ok(found.map_err(|e| reason(&e))?.iter().map(row_of).collect())
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

    fn delete(&self, key: &str) -> Result<Option<String>, SourceError> {
        self.runtime
            .block_on(self.people.delete(id_of(key)?))
            .map(|()| None)
            .map_err(|e| reason(&e))
    }
}
