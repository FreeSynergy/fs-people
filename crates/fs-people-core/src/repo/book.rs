//! `People` implemented by `PeopleRepo`.
//!
//! Pattern: Facade — each trait method forwards to the inherent method of the same name (inherent
//! methods keep working for callers that do not import the trait). A write first takes its turn:
//! edits read a card and put it back, the store can yield in between, and two callers at once must not
//! lose a change (`handle.rs`). The inherent methods compose each other, so the turn is taken here;
//! only an import takes it itself, per card. `update_checked` compares the revision under the same turn.

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_objects::{ObjectId, StoreId, StoredObject};

use super::handle::PeopleRepo;
use super::release::ReleasedExport;
use super::types::{ImportReport, ListFilter};
use crate::card::Card;
use crate::error::{Error, Issue, Result};
use crate::format::{Exporter, Importer};
use crate::matching::SameAsProposal;
use crate::people::People;
use crate::person::PatchObject;
use crate::revision::Revision;

#[async_trait]
impl People for PeopleRepo {
    fn capabilities(&self) -> Capabilities {
        capabilities()
    }
    fn identity(&self) -> Option<StoreId> {
        self.info.as_ref().map(|info| info.identity())
    }
    async fn relate(&self, id: ObjectId, other_uid: &str, relation: &str) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::relate(self, id, other_uid, relation).await
    }
    async fn unrelate(
        &self,
        id: ObjectId,
        other_uid: &str,
        relation: &str,
    ) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::unrelate(self, id, other_uid, relation).await
    }
    async fn relations(&self, id: ObjectId) -> Result<Vec<(String, String)>> {
        PeopleRepo::relations(self, id).await
    }
    async fn patch(&self, id: ObjectId, patch: &PatchObject) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::patch(self, id, patch).await
    }
    async fn check(&self, id: ObjectId) -> Result<Vec<Issue>> {
        PeopleRepo::check(self, id).await
    }
    async fn suggest(&self, field: &str, typed: &str) -> Result<Vec<String>> {
        PeopleRepo::suggest(self, field, typed).await
    }
    async fn create(&self, card: Card) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::create(self, card).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        PeopleRepo::get(self, id).await
    }
    async fn get_card(&self, id: ObjectId) -> Result<Option<Card>> {
        PeopleRepo::get_card(self, id).await
    }
    async fn get_by_uid(&self, uid: &str) -> Result<Option<StoredObject>> {
        PeopleRepo::get_by_uid(self, uid).await
    }
    async fn update(&self, id: ObjectId, card: Card) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::update(self, id, card).await
    }
    async fn update_checked(
        &self,
        id: ObjectId,
        expected: Revision,
        card: Card,
    ) -> Result<StoredObject> {
        // The turn makes compare and write one step: nobody else writes in between.
        let _turn = self.writes.lock().await;
        let stored = PeopleRepo::get(self, id)
            .await?
            .ok_or(Error::NotFound(id))?;
        if Revision::of(&stored) != expected {
            return Err(Error::Changed(id));
        }
        PeopleRepo::update(self, id, card).await
    }
    async fn delete(&self, id: ObjectId) -> Result<()> {
        let _turn = self.writes.lock().await;
        PeopleRepo::delete(self, id).await
    }
    async fn list(&self, filter: &ListFilter) -> Result<Vec<StoredObject>> {
        PeopleRepo::list(self, filter).await
    }
    async fn find_by_describes(&self, iri: &str) -> Result<Vec<StoredObject>> {
        PeopleRepo::find_by_describes(self, iri).await
    }
    async fn find_by_email(&self, email: &str) -> Result<Vec<StoredObject>> {
        PeopleRepo::find_by_email(self, email).await
    }
    async fn find_by_phone(&self, phone: &str) -> Result<Vec<StoredObject>> {
        PeopleRepo::find_by_phone(self, phone).await
    }
    async fn search(&self, term: &str) -> Result<Vec<StoredObject>> {
        PeopleRepo::search(self, term).await
    }
    async fn import(&self, importer: &dyn Importer, text: &str) -> Result<ImportReport> {
        PeopleRepo::import(self, importer, text).await
    }
    async fn export(&self, exporter: &dyn Exporter, id: ObjectId) -> Result<String> {
        PeopleRepo::export(self, exporter, id).await
    }
    async fn export_released(
        &self,
        exporter: &dyn Exporter,
        id: ObjectId,
        released: &[String],
    ) -> Result<ReleasedExport> {
        PeopleRepo::export_released(self, exporter, id, released).await
    }
    async fn create_group(&self, name: &str) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::create_group(self, name).await
    }
    async fn add_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::add_member(self, group, member).await
    }
    async fn remove_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::remove_member(self, group, member).await
    }
    async fn members(&self, group: ObjectId) -> Result<Vec<StoredObject>> {
        PeopleRepo::members(self, group).await
    }
    async fn groups_of(&self, member: ObjectId) -> Result<Vec<StoredObject>> {
        PeopleRepo::groups_of(self, member).await
    }
    async fn find_same_person_proposals(&self) -> Result<Vec<SameAsProposal>> {
        PeopleRepo::find_same_person_proposals(self).await
    }
    async fn confirm_same_person(&self, a: ObjectId, b: ObjectId) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::confirm_same_person(self, a, b).await
    }
    async fn unlink(&self, relationship: ObjectId) -> Result<()> {
        let _turn = self.writes.lock().await;
        PeopleRepo::unlink(self, relationship).await
    }
    async fn reject_same_person(&self, a: ObjectId, b: ObjectId) -> Result<StoredObject> {
        let _turn = self.writes.lock().await;
        PeopleRepo::reject_same_person(self, a, b).await
    }
    async fn reopen_same_person(&self, rejection: ObjectId) -> Result<()> {
        let _turn = self.writes.lock().await;
        PeopleRepo::reopen_same_person(self, rejection).await
    }
    async fn same_person_state(
        &self,
        a: ObjectId,
        b: ObjectId,
    ) -> Result<Option<crate::matching::ProposalState>> {
        PeopleRepo::same_person_state(self, a, b).await
    }
    async fn linked_to(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        PeopleRepo::linked_to(self, id).await
    }
}

/// The store-backed provider offers every capability of the contract.
fn capabilities() -> Capabilities {
    let mut caps = Capabilities::new("fs-people");
    for id in [
        "person.create",
        "person.read",
        "person.update",
        "person.delete",
        "person.check",
        "person.suggest",
        "person.find",
        "person.import",
        "person.export",
        "person.subscribe",
        "relation.read",
        "relation.write",
        "cardgroup.read",
        "cardgroup.write",
        "same.propose",
        "same.link",
        "same.read",
        "role.define",
        "role.configure",
        "role.read",
        "role.assign",
        "ai.mark",
    ] {
        caps.supported(id);
    }
    caps
}
