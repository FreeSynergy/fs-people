//! The [`People`] contract: what the people program offers to everyone else.
//!
//! Pattern: Repository (interface) + Null Object — callers depend on this trait, never on the
//! store-backed `PeopleRepo`. Like a parent class (ADR 0045): every provider has every method. A
//! method it does not override answers an explicit [`Unsupported`], and `capabilities` reports the
//! same. The Rust form of `Persons` in `people.capnp` (fs-interfaces); roles are in [`crate::RoleBook`].

use async_trait::async_trait;
use fs_interfaces::{Capabilities, Unsupported};
use fs_objects::{ObjectId, StoreId, StoredObject};

use crate::card::Card;
use crate::error::{Issue, Result};
use crate::format::{Exporter, Importer, JsContactFormat, VCardFormat};
use crate::matching::{ProposalState, SameAsProposal};
use crate::person::PatchObject;
use crate::repo::{ImportReport, ListFilter, ReleasedExport};
use crate::revision::Revision;

/// The explicit "not possible" of a method a provider does not override.
pub(crate) fn not_possible<T>(capability: &str) -> Result<T> {
    Err(Unsupported::new(capability, "this provider does not offer it").into())
}

/// People: cards, search, import / export, relations, lists and same-person links.
///
/// Interface status: draft (ADR 0045).
#[async_trait]
pub trait People: Send + Sync {
    /// What this provider can do, per capability id.
    fn capabilities(&self) -> Capabilities;
    /// The identity of the store behind this provider, if it has one: made once when the store is
    /// created, never in the log. A provider without a store (a fake) answers `None`.
    fn identity(&self) -> Option<StoreId> {
        None
    }
    /// The stored object of a person, if it exists.
    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>>;
    /// People in name order, filtered and paged.
    async fn list(&self, filter: &ListFilter) -> Result<Vec<StoredObject>>;

    /// `person.create`: validate and store a new card. Refused if its `uid` already exists.
    async fn create(&self, _card: Card) -> Result<StoredObject> {
        not_possible("person.create")
    }
    /// The card of a person, if it exists.
    async fn get_card(&self, _id: ObjectId) -> Result<Option<Card>> {
        not_possible("person.read")
    }
    /// Lookup by the card's `uid`.
    async fn get_by_uid(&self, _uid: &str) -> Result<Option<StoredObject>> {
        not_possible("person.read")
    }
    /// `person.update`: replace the card of `id` (the `uid` cannot change).
    async fn update(&self, _id: ObjectId, _card: Card) -> Result<StoredObject> {
        not_possible("person.update")
    }
    /// `person.update`: change a card by a `PatchObject` (RFC 8984 section 1.4.9): the paths named are set or
    /// removed, everything else stays. All or nothing; the result is validated like any update.
    async fn patch(&self, _id: ObjectId, _patch: &PatchObject) -> Result<StoredObject> {
        not_possible("person.update")
    }
    /// `person.update`, the command `change` of the description: an [`Edit`](fs_types::Edit), field by field,
    /// typed. The provider turns it into the card's PatchObject; all or nothing, validated like an update.
    async fn change(&self, _id: ObjectId, _edit: &fs_types::Edit) -> Result<StoredObject> {
        not_possible("person.update")
    }
    /// `person.check`: the card against RFC 9553 and the rules of its basic types. Reports; changes nothing.
    async fn check(&self, _id: ObjectId) -> Result<Vec<Issue>> {
        not_possible("person.check")
    }
    /// `person.suggest`: completions of `typed` for a field of the person (for an e-mail address: the
    /// domains of the addresses already in the book).
    async fn suggest(&self, _field: &str, _typed: &str) -> Result<Vec<String>> {
        not_possible("person.suggest")
    }
    /// `person.update`, compare-and-set: replace the card only if it still has `expected` (the
    /// [`Revision`] the caller read); otherwise [`Error::Changed`](crate::Error::Changed) and nothing is
    /// written. The check and the write are one step, atomic inside one provider process only.
    async fn update_checked(
        &self,
        _id: ObjectId,
        _expected: Revision,
        _card: Card,
    ) -> Result<StoredObject> {
        not_possible("person.update")
    }
    /// `person.delete`: delete a card, its role assignments and the same-person links that mention it.
    async fn delete(&self, _id: ObjectId) -> Result<()> {
        not_possible("person.delete")
    }
    /// Cards that describe this actor IRI / DID.
    async fn find_by_describes(&self, _iri: &str) -> Result<Vec<StoredObject>> {
        not_possible("person.find")
    }
    /// Exact lookup by e-mail address.
    async fn find_by_email(&self, _email: &str) -> Result<Vec<StoredObject>> {
        not_possible("person.find")
    }
    /// Exact lookup by phone number.
    async fn find_by_phone(&self, _phone: &str) -> Result<Vec<StoredObject>> {
        not_possible("person.find")
    }
    /// Substring search over name, nickname, e-mail, phone and organisation.
    async fn search(&self, _term: &str) -> Result<Vec<StoredObject>> {
        not_possible("person.find")
    }
    /// `person.import`: import the cards `importer` finds in `text`; known `uid`s are skipped.
    async fn import(&self, _importer: &dyn Importer, _text: &str) -> Result<ImportReport> {
        not_possible("person.import")
    }
    /// `person.export`: the card as text of the format `exporter` writes. Roles are never in it.
    async fn export(&self, _exporter: &dyn Exporter, _id: ObjectId) -> Result<String> {
        not_possible("person.export")
    }
    /// `person.export` with a release: a sensitive card only if its `uid` is in `released`, a list
    /// without the sensitive members that are not, naming them. A provider without marks has nothing
    /// to hold back.
    async fn export_released(
        &self,
        exporter: &dyn Exporter,
        id: ObjectId,
        _released: &[String],
    ) -> Result<ReleasedExport> {
        let text = self.export(exporter, id).await?;
        Ok(ReleasedExport {
            text,
            withheld: Vec::new(),
        })
    }
    /// `relation.write`: the card with `other_uid` is `relation` to this one (RFC 9553 §2.1.8).
    async fn relate(
        &self,
        _id: ObjectId,
        _other_uid: &str,
        _relation: &str,
    ) -> Result<StoredObject> {
        not_possible("relation.write")
    }
    /// `relation.write`: remove one relation.
    async fn unrelate(
        &self,
        _id: ObjectId,
        _other_uid: &str,
        _relation: &str,
    ) -> Result<StoredObject> {
        not_possible("relation.write")
    }
    /// `relation.read`: the relations of a card as `(uid, relation)` pairs.
    async fn relations(&self, _id: ObjectId) -> Result<Vec<(String, String)>> {
        not_possible("relation.read")
    }
    /// `cardgroup.write`: create a group card (a list of people; rights live in `fs-groups`).
    async fn create_group(&self, _name: &str) -> Result<StoredObject> {
        not_possible("cardgroup.write")
    }
    /// `cardgroup.write`: add `member` to `group`.
    async fn add_member(&self, _group: ObjectId, _member: ObjectId) -> Result<StoredObject> {
        not_possible("cardgroup.write")
    }
    /// `cardgroup.write`: remove `member` from `group`.
    async fn remove_member(&self, _group: ObjectId, _member: ObjectId) -> Result<StoredObject> {
        not_possible("cardgroup.write")
    }
    /// `cardgroup.read`: resolved members of a group card.
    async fn members(&self, _group: ObjectId) -> Result<Vec<StoredObject>> {
        not_possible("cardgroup.read")
    }
    /// `cardgroup.read`: group cards that contain `member`.
    async fn groups_of(&self, _member: ObjectId) -> Result<Vec<StoredObject>> {
        not_possible("cardgroup.read")
    }
    /// `same.propose`: pairs that are probably the same person (nothing is linked yet).
    async fn find_same_person_proposals(&self) -> Result<Vec<SameAsProposal>> {
        not_possible("same.propose")
    }
    /// `same.link`: link two cards as the same person (`owl:sameAs`); both cards stay.
    async fn confirm_same_person(&self, _a: ObjectId, _b: ObjectId) -> Result<StoredObject> {
        not_possible("same.link")
    }
    /// `same.link`: remove a same-person link.
    async fn unlink(&self, _relationship: ObjectId) -> Result<()> {
        not_possible("same.link")
    }
    /// `same.link`: say two cards are not the same person; the pair is not proposed again (ADR 0058).
    async fn reject_same_person(&self, _a: ObjectId, _b: ObjectId) -> Result<StoredObject> {
        not_possible("same.link")
    }
    /// `same.link`: take a rejection back; the pair may be proposed again.
    async fn reopen_same_person(&self, _rejection: ObjectId) -> Result<()> {
        not_possible("same.link")
    }
    /// `same.read`: where a pair stands: linked, rejected, or `None` (nobody has answered).
    async fn same_person_state(&self, _a: ObjectId, _b: ObjectId) -> Result<Option<ProposalState>> {
        not_possible("same.read")
    }
    /// `same.read`: cards linked to `id` by `owl:sameAs`.
    async fn linked_to(&self, _id: ObjectId) -> Result<Vec<ObjectId>> {
        not_possible("same.read")
    }

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
