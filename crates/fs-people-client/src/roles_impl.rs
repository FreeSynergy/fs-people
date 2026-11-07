//! `RoleBook` over the socket: reading a role and the roles of a person.
//!
//! Pattern: Adapter — each method is one call of `Roles`. Every other method of the trait is not
//! overridden, so it answers the explicit `Unsupported` of the provider rule (ADR 0045).

use async_trait::async_trait;
use fs_people_core::{Result, RoleAssignment, RoleBook, RoleDefinition};
use fs_store::ObjectId;

use crate::RpcPeople;
use crate::calls::roles;

#[async_trait]
impl RoleBook for RpcPeople {
    async fn role(&self, id: &str) -> Result<Option<RoleDefinition>> {
        let id = id.to_string();
        self.call(move |r| async move { roles::get(&r.roles, id).await })
            .await
    }

    async fn roles_of(&self, person: ObjectId) -> Result<Vec<RoleAssignment>> {
        self.call(move |r| async move { roles::roles_of(&r.roles, person).await })
            .await
    }
}
