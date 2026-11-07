//! Role definitions: from configuration and defined by people themselves.
//!
//! Pattern: Repository over a Registry — configuration comes from [`RoleRegistry`], own definitions
//! are objects in the store, so they are logged, undoable and survive a restart. Both look the same.

use fs_store::{Envelope, Query, StoredObject};

use super::consts::{CONTEXT, ROLE_DEFINITION_TYPE};
use super::envelope::stored_value;
use super::handle::PeopleRepo;
use crate::error::{Error, Result};
use crate::roles::{RoleDefinition, RoleDirectory, full_id, stored_forms};

impl PeopleRepo {
    /// Store a role of your own. Refused if the id is already taken.
    pub async fn define_role(&self, mut definition: RoleDefinition) -> Result<()> {
        definition.id = full_id(&definition.id);
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
        if self.registry.find(id).is_some() {
            return Err(Error::Conflict(format!(
                "role {id} comes from configuration"
            )));
        }
        let Some(stored) = self.stored_role(id).await? else {
            return Err(Error::UnknownRole(id.to_string()));
        };
        if !self.people_with_role(&full_id(id), None).await?.is_empty() {
            return Err(Error::Conflict(format!("role {id} is still in use")));
        }
        self.objects.delete(stored.envelope.id, &self.actor).await?;
        Ok(())
    }

    /// Replace the configured roles in memory: checked first, then swapped at once (ADR 0045). What
    /// shall outlive the process goes through `store_role_config`.
    pub fn swap_role_config(&self, json: &str) -> Result<()> {
        self.registry.load(json)
    }

    /// The definition of a role, configured or your own.
    pub async fn role(&self, id: &str) -> Result<Option<RoleDefinition>> {
        if let Some(def) = self.registry.find(id) {
            return Ok(Some(def));
        }
        self.stored_role(id)
            .await?
            .map(|o| stored_def(&o))
            .transpose()
    }

    /// All roles, configured and your own, by id.
    pub async fn roles(&self) -> Result<Vec<RoleDefinition>> {
        let q = Query::new()
            .of_type(ROLE_DEFINITION_TYPE)
            .in_context(CONTEXT);
        let mut out = self.registry.all();
        for o in self.reads.query(&q).await? {
            let own = stored_def(&o)?;
            // Configuration wins, as in `role`: one id, one definition.
            if self.registry.find(&own.id).is_none() {
                out.push(own);
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    pub(super) async fn stored_role(&self, id: &str) -> Result<Option<StoredObject>> {
        // Older data may hold the short id of one of our roles.
        for form in stored_forms(id) {
            let q = Query::new()
                .of_type(ROLE_DEFINITION_TYPE)
                .in_context(CONTEXT)
                .with_index("role_id", form)
                .limit(1);
            if let Some(found) = self.reads.query(&q).await?.into_iter().next() {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
}

/// The definition in a stored object, with the full id.
fn stored_def(o: &StoredObject) -> Result<RoleDefinition> {
    let mut def: RoleDefinition = stored_value(&o.content)?;
    def.id = full_id(&def.id);
    Ok(def)
}
