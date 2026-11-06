//! The [`RoleBook`] contract: role definitions, who holds them, and the marking of AIs.
//!
//! Pattern: Repository (interface) + Null Object — split from `People` so a provider of cards without
//! roles still fits (ADR 0045: it answers an explicit `Unsupported`, ADR 0043: small interfaces). The
//! Rust form of `Roles` in `people.capnp` (fs-interfaces).

use async_trait::async_trait;
use fs_store::ObjectId;
use serde_json::{Map, Value};

use crate::error::Result;
use crate::people::not_possible;
use crate::roles::{RoleAssignment, RoleDefinition};

/// Roles of people (ADR 0052). Interface status: draft (ADR 0045).
#[async_trait]
pub trait RoleBook: Send + Sync {
    /// `role.define`: store a role of your own, defined like every other role.
    async fn define_role(&self, _definition: RoleDefinition) -> Result<()> {
        not_possible("role.define")
    }
    /// `role.define`: remove a role of your own.
    async fn undefine_role(&self, _id: &str) -> Result<()> {
        not_possible("role.define")
    }
    /// `role.configure`: replace the configured roles, checked first and swapped at once. They stay
    /// in force after a restart of the provider.
    async fn load_role_config(&self, _json: &str) -> Result<()> {
        not_possible("role.configure")
    }
    /// `role.read`: the definition of a role.
    async fn role(&self, _id: &str) -> Result<Option<RoleDefinition>> {
        not_possible("role.read")
    }
    /// `role.read`: all roles.
    async fn roles(&self) -> Result<Vec<RoleDefinition>> {
        not_possible("role.read")
    }
    /// `role.assign`: give `person` a role, personally or within the group with this IRI.
    async fn assign_role(
        &self,
        _person: ObjectId,
        _role: &str,
        _scope: Option<&str>,
        _details: Map<String, Value>,
    ) -> Result<RoleAssignment> {
        not_possible("role.assign")
    }
    /// `role.assign`: end an assignment.
    async fn unassign_role(&self, _assignment: ObjectId) -> Result<()> {
        not_possible("role.assign")
    }
    /// `role.read`: the roles of a person.
    async fn roles_of(&self, _person: ObjectId) -> Result<Vec<RoleAssignment>> {
        not_possible("role.read")
    }
    /// `role.read`: the people holding a role in any scope; with `scope`, only within that group.
    async fn people_with_role(&self, _role: &str, _scope: Option<&str>) -> Result<Vec<ObjectId>> {
        not_possible("role.read")
    }
    /// `role.read`: the people holding a role personally, not within a group.
    async fn people_with_personal_role(&self, _role: &str) -> Result<Vec<ObjectId>> {
        not_possible("role.read")
    }
    /// `ai.mark`: mark a card as an AI (`kind` plus role). Restricts nothing (ADR 0054).
    async fn mark_ai(&self, _person: ObjectId) -> Result<RoleAssignment> {
        not_possible("ai.mark")
    }
    /// `ai.mark`: whether this card is marked as an AI.
    async fn is_ai(&self, _person: ObjectId) -> Result<bool> {
        not_possible("ai.mark")
    }
}
