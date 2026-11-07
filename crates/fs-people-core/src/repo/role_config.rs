//! The configured roles across a restart.
//!
//! Pattern: Memento — the last accepted file of each base address is one object in the store, logged
//! and undoable like every other; a start restores them into the registry. Without any, the shipped
//! roles stay. A new file of one base leaves the files of other bases as they are.

use std::collections::BTreeMap;

use fs_store::{Envelope, Query, StoredObject};
use serde_json::Value;

use super::consts::{CONTEXT, ROLE_CONFIG_TYPE};
use super::envelope::stored_value;
use super::handle::PeopleRepo;
use super::role_rules::{check_kind, check_reach};
use super::types::RoleRestore;
use crate::error::{Error, Result};
use crate::roles::{RoleDefinition, RoleScheme, is_ai_role};
use crate::validate::check_exact_numbers;

impl PeopleRepo {
    /// Replace the configured roles of one role file (one base address): checked first, stored, then
    /// swapped at once (ADR 0045). Other files stay. Refused if someone holds a role that would be
    /// gone, or no longer fit what the role asks.
    pub async fn store_role_config(&self, json: &str) -> Result<()> {
        check_exact_numbers(json)?;
        let scheme = RoleScheme::parse(json)?;
        let next = self.registry.preview(json)?;
        self.check_holders_against(&next).await?;
        // The old array adds to what is in force. What is saved, and loaded now, is the file that
        // results, so running and after a restart the same roles hold.
        let resolved = if scheme.merge {
            resolved_file(&scheme.base, &next)?
        } else {
            json.to_string()
        };
        let content: Value = serde_json::from_str(&resolved)?;
        let (previous, _) = self.stored_role_configs().await?;
        let previous = previous.into_iter().find(|(base, _)| *base == scheme.base);
        let env = previous.map_or_else(|| self.role_config_envelope(), |(_, o)| o.envelope);
        let object = StoredObject::new(env, "application/json", content);
        self.objects.put(object, &self.actor).await?;
        self.registry.load(&resolved)
    }

    async fn check_holders_against(&self, next: &BTreeMap<String, RoleDefinition>) -> Result<()> {
        for held in self.all_assignments().await? {
            // Only what this configuration changes counts: an assignment that fits nothing today,
            // from before a rule or a role that went, must not block every configuration after it.
            let Some(now) = self.role(&held.role).await? else {
                continue;
            };
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
            let scope = held.scope.as_deref();
            // An AI is marked without a scope, whatever the role says of its reach.
            if !is_ai_role(&held.role) && check_reach(&now, scope).is_ok() {
                check_reach(&definition, scope)
                    .map_err(|e| Error::Conflict(format!("in use: {e}")))?;
            }
            if let Some(card) = self.get_card(held.person).await? {
                if check_kind(&now, card.kind()).is_ok() {
                    check_kind(&definition, card.kind())?;
                }
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
    /// A stored file that cannot be read is skipped, never fatal; the answer says which and why.
    pub async fn restore_role_config(&self) -> Result<RoleRestore> {
        let (found, mut skipped) = self.stored_role_configs().await?;
        for (_, stored) in found {
            if let Err(e) = self.registry.load_stored(&stored.content.to_string()) {
                skipped.push(format!("role configuration {}: {e}", stored.envelope.id));
            }
        }
        Ok(RoleRestore(skipped))
    }

    /// The stored role files with their base address (the old array form is one of ours), and a
    /// note for each one that cannot be read.
    async fn stored_role_configs(&self) -> Result<(Vec<(String, StoredObject)>, Vec<String>)> {
        let q = Query::new().of_type(ROLE_CONFIG_TYPE).in_context(CONTEXT);
        let (mut out, mut skipped) = (vec![], vec![]);
        for o in self.reads.query(&q).await? {
            match RoleScheme::parse_stored(&o.content.to_string()) {
                Ok(scheme) => out.push((scheme.base, o)),
                Err(e) => skipped.push(format!("role configuration {}: {e}", o.envelope.id)),
            }
        }
        Ok((out, skipped))
    }
}

/// The role file of `base` as it is in `all`: the roles of that base, with short ids.
fn resolved_file(base: &str, all: &BTreeMap<String, RoleDefinition>) -> Result<String> {
    let mut roles = vec![];
    for def in all.values() {
        let Some(short) = def
            .id
            .strip_prefix(base)
            .filter(|s| !s.contains(['/', '#']))
        else {
            continue;
        };
        let mut role = serde_json::to_value(def)?;
        role["id"] = Value::from(short);
        roles.push(role);
    }
    Ok(serde_json::json!({
        "@context": [{"@base": base}],
        "id": "",
        "type": "skos:ConceptScheme",
        "roles": roles,
    })
    .to_string())
}
