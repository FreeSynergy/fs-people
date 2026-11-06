//! The marking of AIs (ADR 0054).
//!
//! Pattern: Repository — an AI is a card of `kind: application` plus the role `ai`. Both together: plain
//! software is `application` too, and a role alone says nothing once the kind has moved on. Marking
//! restricts nothing.

use fs_store::ObjectId;
use serde_json::Map;

use super::handle::PeopleRepo;
use crate::error::{Error, Result};
use crate::roles::{AI_ROLE, RoleAssignment};

const AI_KIND: &str = "application";

impl PeopleRepo {
    /// Mark a card as an AI: JSContact `kind` `application` plus the role (ADR 0054). Refused for a
    /// group card, which stands for several participants, and if another role it holds needs another
    /// kind. Its scope does not count: an AI is one everywhere.
    pub async fn mark_ai(&self, person: ObjectId) -> Result<RoleAssignment> {
        let mut card = self
            .get_card(person)
            .await?
            .ok_or(Error::NotFound(person))?;
        if card.is_group() {
            return Err(Error::Conflict("a group card cannot be an AI".into()));
        }
        let Some(definition) = self.role(AI_ROLE).await? else {
            return Err(Error::Conflict(format!("role {AI_ROLE} is not defined")));
        };
        if card.kind() != AI_KIND {
            self.check_kind_against_roles(person, AI_KIND).await?;
            card.kind = Some(AI_KIND.into());
            self.update(person, card).await?;
        }
        self.hold_role(person, &definition, None, Map::new()).await
    }

    /// Whether this card is marked as an AI: the kind and the role, both.
    pub async fn is_ai(&self, person: ObjectId) -> Result<bool> {
        let Some(card) = self.get_card(person).await? else {
            return Ok(false);
        };
        let roles = self.roles_of(person).await?;
        Ok(card.kind() == AI_KIND && roles.iter().any(|a| a.role == AI_ROLE))
    }
}
