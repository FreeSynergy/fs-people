//! The configured roles across a restart.
//!
//! Pattern: Memento — the last accepted configuration is one object in the store, logged and undoable
//! like every other; a start restores it into the registry. Without one, the shipped roles stay.

use fs_store::{Envelope, Query, StoredObject};
use serde_json::Value;

use super::consts::{CONTEXT, ROLE_CONFIG_TYPE};
use super::handle::PeopleRepo;
use crate::error::{Error, Result};
use crate::roles::RoleRegistry;

impl PeopleRepo {
    /// Replace the configured roles: checked first, stored, then swapped at once (ADR 0045).
    pub async fn store_role_config(&self, json: &str) -> Result<()> {
        RoleRegistry::check(json)?;
        let content: Value = serde_json::from_str(json)?;
        let env = match self.stored_role_config().await? {
            Some(previous) => previous.envelope,
            None => {
                let mut env = Envelope::new(ROLE_CONFIG_TYPE, "Configured roles");
                env.context = Some(CONTEXT.into());
                env.attributed_to = Some(self.actor.clone());
                env.to = vec![self.actor.clone()];
                env.iri = Some(self.iri(env.id));
                env
            }
        };
        let object = StoredObject::new(env, "application/json", content);
        self.objects.put(object, &self.actor).await?;
        self.registry.load(json)
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
