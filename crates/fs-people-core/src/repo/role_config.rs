//! The configured roles across a restart.
//!
//! Pattern: Memento — the last accepted configuration is one object in the store, logged and undoable
//! like every other; a start restores it into the registry. Without one, the shipped roles stay.

use std::collections::BTreeMap;

use fs_store::{Envelope, Query, StoredObject};
use serde_json::Value;

use super::consts::{CONTEXT, ROLE_CONFIG_TYPE};
use super::envelope::stored_value;
use super::handle::PeopleRepo;
use super::role_rules::{check_kind, check_reach};
use crate::error::{Error, Result};
use crate::roles::{RoleDefinition, RoleRegistry};

impl PeopleRepo {
    /// Replace the configured roles: checked first, stored, then swapped at once (ADR 0045). Refused
    /// if someone holds a role that would be gone, or no longer fit what the role asks.
    pub async fn store_role_config(&self, json: &str) -> Result<()> {
        let next = RoleRegistry::check(json)?;
        self.check_holders_against(&next).await?;
        let content: Value = serde_json::from_str(json)?;
        let previous = self.stored_role_config().await?;
        let env = previous.map_or_else(|| self.role_config_envelope(), |o| o.envelope);
        let object = StoredObject::new(env, "application/json", content);
        self.objects.put(object, &self.actor).await?;
        self.registry.load(json)
    }

    async fn check_holders_against(&self, next: &BTreeMap<String, RoleDefinition>) -> Result<()> {
        for held in self.all_assignments().await? {
            let definition = match next.get(&held.role) {
                Some(configured) => configured.clone(),
                None => match self.stored_role(&held.role).await? {
                    Some(own) => stored_value(&own.content)?,
                    None => {
                        return Err(Error::Conflict(format!(
                            "role {} is still in use",
                            held.role
                        )));
                    }
                },
            };
            check_reach(&definition, held.scope.as_deref())
                .map_err(|e| Error::Conflict(format!("in use: {e}")))?;
            if let Some(card) = self.get_card(held.person).await? {
                check_kind(&definition, card.kind())?;
            }
        }
        Ok(())
    }

    fn role_config_envelope(&self) -> Envelope {
        let mut env = Envelope::new(ROLE_CONFIG_TYPE, "Configured roles");
        env.context = Some(CONTEXT.into());
        env.attributed_to = Some(self.actor.clone());
        env.to = vec![self.actor.clone()];
        env.iri = Some(self.iri(env.id));
        env
    }

    /// Put the stored configuration in force, if there is one. Called once when the program starts.
    pub async fn restore_role_config(&self) -> Result<()> {
        let Some(stored) = self.stored_role_config().await? else {
            return Ok(());
        };
        self.registry.load(&stored.content.to_string())
    }

    async fn stored_role_config(&self) -> Result<Option<StoredObject>> {
        let q = Query::new()
            .of_type(ROLE_CONFIG_TYPE)
            .in_context(CONTEXT)
            .limit(1);
        self.reads
            .query(&q)
            .await
            .map(|mut found| found.pop())
            .map_err(Error::from)
    }
}
