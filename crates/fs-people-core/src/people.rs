//! The [`People`] contract: what the people program offers to everyone else.
//!
//! Pattern: Repository (interface) — callers (the GUI, the hub, tests with fakes) depend on this
//! trait, never on the store-backed `PeopleRepo`.

use async_trait::async_trait;
use fs_store::{ObjectId, StoredObject};

use crate::card::Card;
use crate::error::Result;
use crate::format::{Exporter, Importer, JsContactFormat, VCardFormat};
use crate::matching::SameAsProposal;
use crate::repo::{ImportReport, ListFilter};

/// Cards, groups, search, import / export and same-person links.
#[async_trait]
pub trait People: Send + Sync {
    /// Validate and store a new card. Refused if its `uid` already exists.
    async fn create(&self, card: Card) -> Result<StoredObject>;
    /// The stored object of a contact, if it exists.
    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>>;
    /// The card of a contact, if it exists.
    async fn get_card(&self, id: ObjectId) -> Result<Option<Card>>;
    /// Lookup by the card's `uid`.
    async fn get_by_uid(&self, uid: &str) -> Result<Option<StoredObject>>;
    /// Replace the card of `id` (the `uid` cannot change).
    async fn update(&self, id: ObjectId, card: Card) -> Result<StoredObject>;
    /// Delete a card and the same-person links that mention it.
    async fn delete(&self, id: ObjectId) -> Result<()>;
    /// Cards in name order, filtered and paged.
    async fn list(&self, filter: &ListFilter) -> Result<Vec<StoredObject>>;
    /// Cards that describe this actor IRI / DID.
    async fn find_by_describes(&self, iri: &str) -> Result<Vec<StoredObject>>;
    /// Exact lookup by e-mail address.
    async fn find_by_email(&self, email: &str) -> Result<Vec<StoredObject>>;
    /// Exact lookup by phone number.
    async fn find_by_phone(&self, phone: &str) -> Result<Vec<StoredObject>>;
    /// Substring search over name, nickname, e-mail, phone and organisation.
    async fn search(&self, term: &str) -> Result<Vec<StoredObject>>;
    /// Import the cards `importer` finds in `text`; known `uid`s are skipped.
    async fn import(&self, importer: &dyn Importer, text: &str) -> Result<ImportReport>;
    /// The card as text of the format `exporter` writes.
    async fn export(&self, exporter: &dyn Exporter, id: ObjectId) -> Result<String>;
    /// Create a group card.
    async fn create_group(&self, name: &str) -> Result<StoredObject>;
    /// Add `member` to `group`.
    async fn add_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject>;
    /// Remove `member` from `group`.
    async fn remove_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject>;
    /// Resolved members of a group.
    async fn members(&self, group: ObjectId) -> Result<Vec<StoredObject>>;
    /// Groups that contain `member`.
    async fn groups_of(&self, member: ObjectId) -> Result<Vec<StoredObject>>;
    /// Pairs that are probably the same person (nothing is linked yet).
    async fn find_same_person_proposals(&self) -> Result<Vec<SameAsProposal>>;
    /// Link two cards as the same person (`owl:sameAs`); both cards stay.
    async fn confirm_same_person(&self, a: ObjectId, b: ObjectId) -> Result<StoredObject>;
    /// Remove a same-person link.
    async fn unlink(&self, relationship: ObjectId) -> Result<()>;
    /// Cards linked to `id` by `owl:sameAs`.
    async fn linked_to(&self, id: ObjectId) -> Result<Vec<ObjectId>>;

    /// Import a `.vcf` text.
    async fn import_vcard(&self, text: &str) -> Result<ImportReport> {
        self.import(&VCardFormat, text).await
    }
    /// Import JSContact JSON (one card or an array).
    async fn import_jscontact(&self, text: &str) -> Result<ImportReport> {
        self.import(&JsContactFormat, text).await
    }
    /// The card as vCard 4.0 text.
    async fn export_vcard(&self, id: ObjectId) -> Result<String> {
        self.export(&VCardFormat, id).await
    }
}
