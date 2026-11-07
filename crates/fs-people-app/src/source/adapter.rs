//! `PeopleAdapter`: `dyn People` as a `PeopleSource`.
//!
//! Pattern: Adapter over the program's contract (`People` and `RoleBook`, in practice the client of
//! `people.capnp`), driven with a tokio handle from the interface thread: every call blocks that
//! thread until the server has answered (`DESIGN.md`).
//!
//! Revision: the one the server gives (`RevisedPeople`, `Person.revision`), kept in the draft as
//! text. `save` hands it to `People::update_checked`, so the check and the write are one step in the
//! provider; nobody can slip in between. The app cannot derive the revision itself: the wire carries
//! time in milliseconds, the store keeps microseconds.

use std::sync::{Arc, Mutex, PoisonError};

use fs_components_workbenches::SourceError;
use fs_components_workbenches::people::{
    PeopleCapabilities, PeopleSource, PersonDraft, PersonRow, PersonSheet,
};
use fs_people_client::{PersonPages, RevisedPeople};
use fs_people_core::{Card, Error, People, Revision, RoleBook, card_of};
use fs_objects::{ObjectId, StoredObject};
use tokio::runtime::Handle;

use super::apply::apply_draft;
use super::draft::{blank_draft, draft_of};
use super::reading::Reading;
use super::sheet::{row_of, sensitive_mark, sheet_of};
use crate::words::{fill, said};

/// The contract of the people program, for the workbench. Do not call it from inside the runtime:
/// it blocks the calling thread until the provider has answered.
pub struct PeopleAdapter {
    people: Arc<dyn People>,
    roles: Arc<dyn RoleBook>,
    pages: Arc<dyn PersonPages>,
    revised: Arc<dyn RevisedPeople>,
    reading: Mutex<Option<Reading>>,
    runtime: Handle,
}

impl PeopleAdapter {
    /// An adapter over a provider of both contracts, of the page-wise reading and of the revision,
    /// driven with `runtime`.
    pub fn new(
        people: Arc<dyn People>,
        roles: Arc<dyn RoleBook>,
        pages: Arc<dyn PersonPages>,
        revised: Arc<dyn RevisedPeople>,
        runtime: Handle,
    ) -> Self {
        Self {
            people,
            roles,
            pages,
            revised,
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
            .ok_or_else(|| SourceError::not_found(said::GONE))
    }
}

/// The key of a row: the object id.
fn id_of(key: &str) -> Result<ObjectId, SourceError> {
    key.parse()
        .map_err(|_| SourceError::not_found(fill(said::NAMES_NOBODY, &[("key", key)])))
}

/// What the core said, in the workbench's terms. A store refusing a sensitive card is said plainly.
fn reason(error: &Error) -> SourceError {
    match error {
        Error::Store(fs_objects::Error::EncryptionRequired(_)) => SourceError::failed(fill(
            said::NEEDS_ENCRYPTION,
            &[("error", &error.to_string())],
        )),
        Error::Changed(_) => SourceError::conflict(said::CHANGED),
        Error::NotFound(_) => SourceError::not_found(error.to_string()),
        Error::Unsupported(_) => SourceError::unsupported(error.to_string()),
        _ => SourceError::failed(error.to_string()),
    }
}

/// The same reason as plain text.
pub(crate) fn reason_text(error: &Error) -> String {
    reason(error).message
}

/// The revision as the draft holds it: the text of [`Revision`], read back.
fn revision_in(draft: &PersonDraft) -> Option<Revision> {
    let text = draft.revision.as_deref()?;
    u64::from_str_radix(text, 16).ok().map(Revision::from)
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
        let read = self
            .runtime
            .block_on(self.revised.get_revised(id_of(key)?))
            .map_err(|e| reason(&e))?
            .ok_or_else(|| SourceError::not_found(said::GONE))?;
        let card = card_of(&read.object).map_err(|e| reason(&e))?;
        let mut draft = draft_of(&card, Some(read.revision.to_string()));
        draft.marks.extend(sensitive_mark(&card, &read.object));
        Ok(draft)
    }

    fn blank(&self) -> Result<PersonDraft, SourceError> {
        Ok(blank_draft())
    }

    /// The form goes onto the card as it is stored now, so whatever the form does not show stays.
    fn save(&self, key: &str, draft: &PersonDraft) -> Result<(), SourceError> {
        let id = id_of(key)?;
        let object = self.stored(id)?;
        let expected = revision_in(draft).ok_or_else(|| SourceError::conflict(said::CHANGED))?;
        let mut card = card_of(&object).map_err(|e| reason(&e))?;
        apply_draft(&mut card, draft)?;
        self.runtime
            .block_on(self.people.update_checked(id, expected, card))
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
