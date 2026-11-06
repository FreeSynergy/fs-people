//! Who holds which role, and the marking of AIs.
//!
//! Pattern: Repository — an assignment is an AS2 `Relationship` (person -> role, in a scope) of its
//! own, like a same-person link: logged, undoable, and never part of a card or its export (ADR 0029).
//! Marking an AI is `kind` plus role and restricts nothing (ADR 0054).

use fs_store::{Envelope, ObjectId, Query, StoredObject};
use serde_json::{Map, Value, json};

use super::consts::{AS2_TYPE, CONTEXT, RELATIONSHIP_TYPE};
use super::handle::PeopleRepo;
use crate::error::{Error, Issue, Result};
use crate::roles::{AI_ROLE, RoleAssignment};

const PERSONAL: &str = "personal";
/// The owner whose role fields this program holds itself.
const OWN_ROLES: &str = "fs-people";

impl PeopleRepo {
    /// Give `person` the role, personally (`scope` = `None`) or within the group with this IRI.
    /// Holding it already changes nothing and returns the assignment. Refused for an unknown role,
    /// and if the role needs another JSContact `kind`.
    pub async fn assign_role(
        &self,
        person: ObjectId,
        role: &str,
        scope: Option<&str>,
        details: Map<String, Value>,
    ) -> Result<RoleAssignment> {
        if let Some(s) = scope.filter(|s| !s.contains(':')) {
            return Err(Error::Invalid(vec![Issue {
                path: "/scope".into(),
                message: format!("{s:?} is not the IRI of a group"),
            }]));
        }
        let definition = self
            .role(role)
            .await?
            .ok_or_else(|| Error::Conflict(format!("role {role} is not defined")))?;
        let card = self
            .get_card(person)
            .await?
            .ok_or(Error::NotFound(person))?;
        if let Some(kind) = &definition.card_kind {
            if card.kind() != kind {
                return Err(Error::Conflict(format!(
                    "role {role} needs a card of kind {kind}, this one is {}",
                    card.kind()
                )));
            }
        }
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
        if !found.as_ref().is_some_and(|o| read(o).is_some()) {
            return Err(fs_store::Error::NotFound(assignment).into());
        }
        self.objects.delete(assignment, &self.actor).await?;
        Ok(())
    }

    /// The roles of a person.
    pub async fn roles_of(&self, person: ObjectId) -> Result<Vec<RoleAssignment>> {
        let q = assignments().with_index("person", person.to_string());
        Ok(self
            .reads
            .query(&q)
            .await?
            .iter()
            .filter_map(read)
            .collect())
    }

    /// The people holding a role; with `scope`, only within that group.
    pub async fn people_with_role(&self, role: &str, scope: Option<&str>) -> Result<Vec<ObjectId>> {
        let mut q = assignments().with_index("role", role);
        if let Some(s) = scope {
            q = q.with_index("scope", s);
        }
        let found = self.reads.query(&q).await?;
        let mut holders: Vec<ObjectId> = found.iter().filter_map(read).map(|a| a.person).collect();
        holders.sort_unstable();
        holders.dedup(); // one person can hold a role in several scopes
        Ok(holders)
    }

    /// Mark a card as an AI: JSContact `kind` `application` plus the role (ADR 0054). Refused for a
    /// group card, which stands for several participants.
    pub async fn mark_ai(&self, person: ObjectId) -> Result<RoleAssignment> {
        let mut card = self
            .get_card(person)
            .await?
            .ok_or(Error::NotFound(person))?;
        if card.is_group() {
            return Err(Error::Conflict("a group card cannot be an AI".into()));
        }
        if self.role(AI_ROLE).await?.is_none() {
            return Err(Error::Conflict(format!("role {AI_ROLE} is not defined")));
        }
        if card.kind() != "application" {
            card.kind = Some("application".into());
            self.update(person, card).await?;
        }
        self.assign_role(person, AI_ROLE, None, Map::new()).await
    }

    /// Whether this card is marked as an AI.
    pub async fn is_ai(&self, person: ObjectId) -> Result<bool> {
        let roles = self.roles_of(person).await?;
        Ok(roles.iter().any(|a| a.role == AI_ROLE))
    }

    pub(super) async fn delete_assignments_of(&self, person: ObjectId) -> Result<()> {
        for a in self.roles_of(person).await? {
            self.objects.delete(a.id, &self.actor).await?;
        }
        Ok(())
    }
}

fn assignments() -> Query {
    Query::new().of_type(RELATIONSHIP_TYPE).in_context(CONTEXT)
}

/// The assignment in `o`, if it is one; a same-person link or a card is not.
fn read(o: &StoredObject) -> Option<RoleAssignment> {
    if o.envelope.object_type != RELATIONSHIP_TYPE {
        return None;
    }
    let index = |k: &str| o.index.get(k).and_then(|v| v.first());
    let scope = index("scope")?;
    Some(RoleAssignment {
        id: o.envelope.id,
        person: index("person")?.parse().ok()?,
        role: index("role")?.clone(),
        scope: (scope != PERSONAL).then(|| scope.clone()),
        details: o
            .content
            .get("details")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
    })
}
