//! `People` implemented by `PeopleRepo`.
//!
//! Pattern: Facade — each trait method forwards to the inherent method of the same name (inherent
//! methods keep working for callers that do not import the trait).

use async_trait::async_trait;
use fs_store::{ObjectId, StoredObject};

use super::handle::PeopleRepo;
use super::types::{ImportReport, ListFilter};
use crate::card::Card;
use crate::error::Result;
use crate::format::{Exporter, Importer};
use crate::matching::SameAsProposal;
use crate::people::People;

#[async_trait]
impl People for PeopleRepo {
    async fn create(&self, card: Card) -> Result<StoredObject> {
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
        PeopleRepo::update(self, id, card).await
    }
    async fn delete(&self, id: ObjectId) -> Result<()> {
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
    async fn create_group(&self, name: &str) -> Result<StoredObject> {
        PeopleRepo::create_group(self, name).await
    }
    async fn add_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        PeopleRepo::add_member(self, group, member).await
    }
    async fn remove_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
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
        PeopleRepo::confirm_same_person(self, a, b).await
    }
    async fn unlink(&self, relationship: ObjectId) -> Result<()> {
        PeopleRepo::unlink(self, relationship).await
    }
    async fn linked_to(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        PeopleRepo::linked_to(self, id).await
    }
}
