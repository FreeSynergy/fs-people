//! `RoleBook` and `PeopleFeed` implemented by `PeopleRepo`.
//!
//! Pattern: Facade — each trait method forwards to the inherent method of the same name; a write first
//! takes its turn, as in `book.rs`.

use std::sync::Arc;

use async_trait::async_trait;
use fs_store::ObjectId;
use serde_json::{Map, Value};

use super::handle::PeopleRepo;
use crate::error::Result;
use crate::feed::{PeopleFeed, PersonListener, Subscription};
use crate::role_book::RoleBook;
use crate::roles::{RoleAssignment, RoleDefinition};

#[async_trait]
impl RoleBook for PeopleRepo {
    async fn define_role(&self, definition: RoleDefinition) -> Result<()> {
        let _turn = self.writes.lock().await;
        PeopleRepo::define_role(self, definition).await
    }
    async fn undefine_role(&self, id: &str) -> Result<()> {
        let _turn = self.writes.lock().await;
        PeopleRepo::undefine_role(self, id).await
    }
    async fn load_role_config(&self, json: &str) -> Result<()> {
        let _turn = self.writes.lock().await;
        PeopleRepo::store_role_config(self, json).await
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
        let _turn = self.writes.lock().await;
        PeopleRepo::assign_role(self, person, role, scope, details).await
    }
    async fn unassign_role(&self, assignment: ObjectId) -> Result<()> {
        let _turn = self.writes.lock().await;
        PeopleRepo::unassign_role(self, assignment).await
    }
    async fn roles_of(&self, person: ObjectId) -> Result<Vec<RoleAssignment>> {
        PeopleRepo::roles_of(self, person).await
    }
    async fn people_with_role(&self, role: &str, scope: Option<&str>) -> Result<Vec<ObjectId>> {
        PeopleRepo::people_with_role(self, role, scope).await
    }
    async fn people_with_personal_role(&self, role: &str) -> Result<Vec<ObjectId>> {
        PeopleRepo::people_with_personal_role(self, role).await
    }
    async fn mark_ai(&self, person: ObjectId) -> Result<RoleAssignment> {
        let _turn = self.writes.lock().await;
        PeopleRepo::mark_ai(self, person).await
    }
    async fn is_ai(&self, person: ObjectId) -> Result<bool> {
        PeopleRepo::is_ai(self, person).await
    }
}

impl PeopleFeed for PeopleRepo {
    fn subscribe(&self, listener: Arc<dyn PersonListener>) -> Result<Subscription> {
        Ok(self.feed.add(listener))
    }
}
