//! `ContactBook` implemented by `ContactRepo`.
//!
//! Pattern: Facade — each trait method forwards to the inherent method of the same name (inherent
//! methods keep working for callers that do not import the trait).

use async_trait::async_trait;
use fs_store::{ObjectId, StoredObject};

use super::handle::ContactRepo;
use super::types::{ImportReport, ListFilter};
use crate::book::ContactBook;
use crate::card::Card;
use crate::error::Result;
use crate::format::{Exporter, Importer};
use crate::matching::SameAsProposal;

#[async_trait]
impl ContactBook for ContactRepo {
    async fn create(&self, card: Card) -> Result<StoredObject> {
        ContactRepo::create(self, card).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        ContactRepo::get(self, id).await
    }
    async fn get_card(&self, id: ObjectId) -> Result<Option<Card>> {
        ContactRepo::get_card(self, id).await
    }
    async fn get_by_uid(&self, uid: &str) -> Result<Option<StoredObject>> {
        ContactRepo::get_by_uid(self, uid).await
    }
    async fn update(&self, id: ObjectId, card: Card) -> Result<StoredObject> {
        ContactRepo::update(self, id, card).await
    }
    async fn delete(&self, id: ObjectId) -> Result<()> {
        ContactRepo::delete(self, id).await
    }
    async fn list(&self, filter: &ListFilter) -> Result<Vec<StoredObject>> {
        ContactRepo::list(self, filter).await
    }
    async fn find_by_describes(&self, iri: &str) -> Result<Vec<StoredObject>> {
        ContactRepo::find_by_describes(self, iri).await
    }
    async fn find_by_email(&self, email: &str) -> Result<Vec<StoredObject>> {
        ContactRepo::find_by_email(self, email).await
    }
    async fn find_by_phone(&self, phone: &str) -> Result<Vec<StoredObject>> {
        ContactRepo::find_by_phone(self, phone).await
    }
    async fn search(&self, term: &str) -> Result<Vec<StoredObject>> {
        ContactRepo::search(self, term).await
    }
    async fn import(&self, importer: &dyn Importer, text: &str) -> Result<ImportReport> {
        ContactRepo::import(self, importer, text).await
    }
    async fn export(&self, exporter: &dyn Exporter, id: ObjectId) -> Result<String> {
        ContactRepo::export(self, exporter, id).await
    }
    async fn create_group(&self, name: &str) -> Result<StoredObject> {
        ContactRepo::create_group(self, name).await
    }
    async fn add_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        ContactRepo::add_member(self, group, member).await
    }
    async fn remove_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        ContactRepo::remove_member(self, group, member).await
    }
    async fn members(&self, group: ObjectId) -> Result<Vec<StoredObject>> {
        ContactRepo::members(self, group).await
    }
    async fn groups_of(&self, member: ObjectId) -> Result<Vec<StoredObject>> {
        ContactRepo::groups_of(self, member).await
    }
    async fn find_same_person_proposals(&self) -> Result<Vec<SameAsProposal>> {
        ContactRepo::find_same_person_proposals(self).await
    }
    async fn confirm_same_person(&self, a: ObjectId, b: ObjectId) -> Result<StoredObject> {
        ContactRepo::confirm_same_person(self, a, b).await
    }
    async fn unlink(&self, relationship: ObjectId) -> Result<()> {
        ContactRepo::unlink(self, relationship).await
    }
    async fn linked_to(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        ContactRepo::linked_to(self, id).await
    }
}
