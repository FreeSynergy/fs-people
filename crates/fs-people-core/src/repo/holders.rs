//! Who holds a role: the questions asked of the assignments.
//!
//! Pattern: Repository (queries) — an assignment stores the full id of its role. Data from before
//! may hold the short id of one of our roles, so a question asks for every stored form and a read
//! always answers the full id; an AI is recognised either way.

use std::collections::BTreeSet;

use fs_objects::{ObjectId, Query, StoredObject};
use serde_json::Value;

use super::consts::{CONTEXT, RELATIONSHIP_TYPE};
use super::handle::PeopleRepo;
use super::role_rules::check_iri;
use crate::error::Result;
use crate::roles::{RoleAssignment, full_id, stored_forms};

pub(super) const PERSONAL: &str = "personal";

impl PeopleRepo {
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

    /// The people holding a role in any scope; with `scope`, only within that group. `role` is the
    /// full id, or the short id of one of our roles.
    pub async fn people_with_role(&self, role: &str, scope: Option<&str>) -> Result<Vec<ObjectId>> {
        check_iri(scope)?;
        self.holders(role, scope).await
    }

    /// The people holding a role personally, not within a group.
    pub async fn people_with_personal_role(&self, role: &str) -> Result<Vec<ObjectId>> {
        self.holders(role, Some(PERSONAL)).await
    }

    /// The assignments held within the group with this IRI.
    pub(super) async fn assignments_scoped_to(&self, iri: &str) -> Result<Vec<ObjectId>> {
        let q = assignments().with_index("scope", iri);
        let found = self.reads.query(&q).await?;
        Ok(found.iter().filter_map(read).map(|a| a.id).collect())
    }

    /// Every assignment there is.
    pub(super) async fn all_assignments(&self) -> Result<Vec<RoleAssignment>> {
        let found = self.reads.query(&assignments()).await?;
        Ok(found.iter().filter_map(read).collect())
    }

    async fn holders(&self, role: &str, scope: Option<&str>) -> Result<Vec<ObjectId>> {
        let mut people = BTreeSet::new();
        for form in stored_forms(role) {
            let mut q = assignments().with_index("role", form);
            if let Some(s) = scope {
                q = q.with_index("scope", s);
            }
            let found = self.reads.query(&q).await?;
            people.extend(found.iter().filter_map(read).map(|a| a.person));
        }
        Ok(people.into_iter().collect()) // one person can hold a role in several scopes
    }
}

pub(super) fn assignments() -> Query {
    Query::new().of_type(RELATIONSHIP_TYPE).in_context(CONTEXT)
}

/// The assignment in `o`, if it is one; a same-person link or a card is not.
pub(super) fn read(o: &StoredObject) -> Option<RoleAssignment> {
    if o.envelope.object_type != RELATIONSHIP_TYPE {
        return None;
    }
    let index = |k: &str| o.index.get(k).and_then(|v| v.first());
    let scope = index("scope")?;
    Some(RoleAssignment {
        id: o.envelope.id,
        person: index("person")?.parse().ok()?,
        role: full_id(index("role")?),
        scope: (scope != PERSONAL).then(|| scope.clone()),
        details: o
            .content
            .get("details")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
    })
}
