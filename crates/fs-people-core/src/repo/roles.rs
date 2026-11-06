//! Role definitions: from configuration and defined by people themselves.
//!
//! Pattern: Repository over a Registry — configuration comes from [`RoleRegistry`], own definitions
//! are objects in the store, so they are logged, undoable and survive a restart. Both look the same.

use fs_store::{Envelope, Query, StoredObject};

use super::consts::{CONTEXT, ROLE_DEFINITION_TYPE};
use super::envelope::stored_value;
use super::handle::PeopleRepo;
use crate::error::{Error, Result};
use crate::roles::RoleDefinition;

impl PeopleRepo {
    /// Store a role of your own. Refused if the id is already taken.
    pub async fn define_role(&self, definition: RoleDefinition) -> Result<()> {
        definition.check()?;
        if self.role(&definition.id).await?.is_some() {
            return Err(Error::Conflict(format!(
                "role {} is already defined",
                definition.id
            )));
        }
        let name = definition
            .names
            .values()
            .next()
            .cloned()
            .unwrap_or_else(|| definition.id.clone());
        let mut env = Envelope::new(ROLE_DEFINITION_TYPE, name);
        env.context = Some(CONTEXT.into());
        env.attributed_to = Some(self.actor.clone());
        env.to = vec![self.actor.clone()];
        env.iri = Some(self.iri(env.id));
        let obj = StoredObject::new(env, "application/json", serde_json::to_value(&definition)?)
            .with_index("role_id", definition.id);
        self.objects.put(obj, &self.actor).await?;
        Ok(())
    }

    /// Remove a role of your own. Refused while someone holds it, or if it comes from configuration.
    pub async fn undefine_role(&self, id: &str) -> Result<()> {
        if self.registry.snapshot().contains_key(id) {
            return Err(Error::Conflict(format!(
                "role {id} comes from configuration"
            )));
        }
        let Some(stored) = self.stored_role(id).await? else {
            return Err(Error::UnknownRole(id.to_string()));
        };
        if !self.people_with_role(id, None).await?.is_empty() {
            return Err(Error::Conflict(format!("role {id} is still in use")));
        }
        self.objects.delete(stored.envelope.id, &self.actor).await?;
        Ok(())
    }

    /// Replace the configured roles in memory: checked first, then swapped at once (ADR 0045). What
    /// shall outlive the process goes through `store_role_config`.
    pub fn load_role_config(&self, json: &str) -> Result<()> {
        self.registry.load(json)
    }

    /// The definition of a role, configured or your own.
    pub async fn role(&self, id: &str) -> Result<Option<RoleDefinition>> {
        if let Some(def) = self.registry.snapshot().get(id) {
            return Ok(Some(def.clone()));
        }
        self.stored_role(id)
            .await?
            .map(|o| stored_value(&o.content))
            .transpose()
    }

    /// All roles, configured and your own, by id.
    pub async fn roles(&self) -> Result<Vec<RoleDefinition>> {
        let q = Query::new()
            .of_type(ROLE_DEFINITION_TYPE)
            .in_context(CONTEXT);
        let configured = self.registry.snapshot();
        let mut out: Vec<RoleDefinition> = configured.values().cloned().collect();
        for o in self.reads.query(&q).await? {
            let own: RoleDefinition = stored_value(&o.content)?;
            // Configuration wins, as in `role`: one id, one definition.
            if !configured.contains_key(&own.id) {
                out.push(own);
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    pub(super) async fn stored_role(&self, id: &str) -> Result<Option<StoredObject>> {
        let q = Query::new()
            .of_type(ROLE_DEFINITION_TYPE)
            .in_context(CONTEXT)
            .with_index("role_id", id)
            .limit(1);
        Ok(self.reads.query(&q).await?.into_iter().next())
    }
}
