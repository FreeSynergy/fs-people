//! Who holds which role, and the marking of AIs.
//!
//! Pattern: Repository — an assignment is an AS2 `Relationship` (person -> role, in a scope) of its
//! own, like a same-person link: logged, undoable, and never part of a card or its export (ADR 0029).

use fs_store::{Envelope, ObjectId, StoredObject};
use serde_json::{Map, Value, json};

use super::consts::{AS2_TYPE, CONTEXT, RELATIONSHIP_TYPE};
use super::handle::PeopleRepo;
use super::holders::{PERSONAL, read};
use super::role_rules::{check_iri, check_kind, check_reach};
use crate::error::{Error, Issue, Result};
use crate::roles::{RoleAssignment, RoleDefinition};

/// The owner whose role fields this program holds itself.
const OWN_ROLES: &str = "fs-people";

impl PeopleRepo {
    /// Give `person` the role, personally (`scope` = `None`) or within the group with this IRI, as
    /// far as the role reaches. Holding it already changes nothing and returns the assignment; other
    /// `details` than it holds are a conflict, not dropped: end the assignment first. Refused for an
    /// unknown role, and if the role needs another JSContact `kind`.
    pub async fn assign_role(
        &self,
        person: ObjectId,
        role: &str,
        scope: Option<&str>,
        details: Map<String, Value>,
    ) -> Result<RoleAssignment> {
        check_iri(scope)?;
        let definition = self
            .role(role)
            .await?
            .ok_or_else(|| Error::Conflict(format!("role {role} is not defined")))?;
        check_reach(&definition, scope)?;
        self.hold_role(person, &definition, scope, details).await
    }

    /// Store that `person` holds the role, after what any way of getting it has to meet.
    pub(super) async fn hold_role(
        &self,
        person: ObjectId,
        definition: &RoleDefinition,
        scope: Option<&str>,
        details: Map<String, Value>,
    ) -> Result<RoleAssignment> {
        let role = definition.id.as_str();
        let card = self
            .get_card(person)
            .await?
            .ok_or(Error::NotFound(person))?;
        check_kind(definition, card.kind())?;
        if !details.is_empty() && definition.owner != OWN_ROLES {
            return Err(Error::Invalid(vec![Issue {
                path: "/details".into(),
                message: format!(
                    "role {role} is owned by {}, which holds its fields",
                    definition.owner
                ),
            }]));
        }
        if let Some(held) = self
            .roles_of(person)
            .await?
            .into_iter()
            .find(|a| a.role == role && a.scope.as_deref() == scope)
        {
            if !details.is_empty() && details != held.details {
                return Err(Error::Conflict(format!(
                    "role {role} is held with other details; end the assignment first"
                )));
            }
            return Ok(held);
        }
        let mut env = Envelope::new(
            RELATIONSHIP_TYPE,
            format!("{} is {role}", card.display_name()),
        );
        env.context = Some(CONTEXT.into());
        env.attributed_to = Some(self.actor.clone());
        env.to = vec![self.actor.clone()];
        env.iri = Some(self.iri(env.id));
        let content = json!({
            "@context": ["https://www.w3.org/ns/activitystreams", {"org": "http://www.w3.org/ns/org#"}],
            "type": RELATIONSHIP_TYPE,
            "subject": self.iri(person),
            "relationship": role,
            "object": scope,
            "details": details,
        });
        let id = env.id;
        let obj = StoredObject::new(env, AS2_TYPE, content)
            .with_index("role", role)
            .with_index("person", person.to_string())
            .with_index("scope", scope.unwrap_or(PERSONAL));
        self.objects.put(obj, &self.actor).await?;
        Ok(RoleAssignment {
            id,
            person,
            role: role.into(),
            scope: scope.map(str::to_string),
            details,
        })
    }

    /// End an assignment (the store's undo can bring it back). Any other object with this id stays.
    pub async fn unassign_role(&self, assignment: ObjectId) -> Result<()> {
        let found = self.objects.get(assignment).await?;
        if found.as_ref().and_then(read).is_none() {
            return Err(fs_store::Error::NotFound(assignment).into());
        }
        self.objects.delete(assignment, &self.actor).await?;
        Ok(())
    }

    pub(super) async fn delete_assignments_of(&self, person: ObjectId) -> Result<()> {
        for a in self.roles_of(person).await? {
            self.objects.delete(a.id, &self.actor).await?;
        }
        Ok(())
    }
}
