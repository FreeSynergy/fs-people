//! `People` and `RoleBook` implemented by `PeopleRepo`.
//!
//! Pattern: Facade — each trait method forwards to the inherent method of the same name (inherent
//! methods keep working for callers that do not import the trait).

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_store::{ObjectId, StoredObject};
use serde_json::{Map, Value};

use super::handle::PeopleRepo;
use super::types::{ImportReport, ListFilter};
use crate::card::Card;
use crate::error::Result;
use crate::format::{Exporter, Importer};
use crate::matching::SameAsProposal;
use crate::people::People;
use crate::role_book::RoleBook;
use crate::roles::{RoleAssignment, RoleDefinition};

#[async_trait]
impl People for PeopleRepo {
    fn capabilities(&self) -> Capabilities {
        capabilities()
    }
    async fn relate(&self, id: ObjectId, other_uid: &str, relation: &str) -> Result<StoredObject> {
        PeopleRepo::relate(self, id, other_uid, relation).await
    }
    async fn unrelate(
        &self,
        id: ObjectId,
        other_uid: &str,
        relation: &str,
    ) -> Result<StoredObject> {
        PeopleRepo::unrelate(self, id, other_uid, relation).await
    }
    async fn relations(&self, id: ObjectId) -> Result<Vec<(String, String)>> {
        PeopleRepo::relations(self, id).await
    }
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

#[async_trait]
impl RoleBook for PeopleRepo {
    async fn define_role(&self, definition: RoleDefinition) -> Result<()> {
        PeopleRepo::define_role(self, definition).await
    }
    async fn undefine_role(&self, id: &str) -> Result<()> {
        PeopleRepo::undefine_role(self, id).await
    }
    async fn load_role_config(&self, json: &str) -> Result<()> {
        PeopleRepo::load_role_config(self, json)
    }
    async fn role(&self, id: &str) -> Result<Option<RoleDefinition>> {
        PeopleRepo::role(self, id).await
    }
    async fn roles(&self) -> Result<Vec<RoleDefinition>> {
        PeopleRepo::roles(self).await
    }
    async fn assign_role(
        &self,
        person: ObjectId,
        role: &str,
        scope: Option<&str>,
        details: Map<String, Value>,
    ) -> Result<RoleAssignment> {
        PeopleRepo::assign_role(self, person, role, scope, details).await
    }
    async fn unassign_role(&self, assignment: ObjectId) -> Result<()> {
        PeopleRepo::unassign_role(self, assignment).await
    }
    async fn roles_of(&self, person: ObjectId) -> Result<Vec<RoleAssignment>> {
        PeopleRepo::roles_of(self, person).await
    }
    async fn people_with_role(&self, role: &str, scope: Option<&str>) -> Result<Vec<ObjectId>> {
        PeopleRepo::people_with_role(self, role, scope).await
    }
    async fn mark_ai(&self, person: ObjectId) -> Result<RoleAssignment> {
        PeopleRepo::mark_ai(self, person).await
    }
    async fn is_ai(&self, person: ObjectId) -> Result<bool> {
        PeopleRepo::is_ai(self, person).await
    }
}

/// The store-backed provider offers every method, except the live feed, which is not built yet.
fn capabilities() -> Capabilities {
    let mut caps = Capabilities::new("fs-people");
    for id in [
        "person.create",
        "person.read",
        "person.update",
        "person.delete",
        "person.find",
        "person.import",
        "person.export",
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
    caps.unsupported("person.subscribe", "not built yet; it is possible");
    caps
}
