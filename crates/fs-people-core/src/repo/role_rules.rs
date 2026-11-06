//! What a role asks of whoever holds it: the same rules at every door.
//!
//! Pattern: Specification — `assign`, `update`, `mark_ai` and a new configuration all ask here, so a
//! card cannot leave through one what it had to meet at another.

use fs_store::ObjectId;

use super::handle::PeopleRepo;
use crate::error::{Error, Issue, Result};
use crate::roles::{RoleDefinition, RoleScope};

fn scope_issue(message: String) -> Error {
    Error::Invalid(vec![Issue {
        path: "/scope".into(),
        message,
    }])
}

/// A scope that is named has to be an IRI, which also keeps it apart from the internal "personal".
pub(super) fn check_iri(scope: Option<&str>) -> Result<()> {
    match scope.filter(|s| !s.contains(':')) {
        Some(s) => Err(scope_issue(format!("{s:?} is not the IRI of a group"))),
        None => Ok(()),
    }
}

/// How far the role reaches decides whether it can have a group: `group` needs one, `personal` has
/// none, `public` can be either.
pub(super) fn check_reach(definition: &RoleDefinition, scope: Option<&str>) -> Result<()> {
    let id = &definition.id;
    match (definition.scope, scope) {
        (RoleScope::Group, None) => Err(scope_issue(format!(
            "role {id} is held within a group: name its IRI"
        ))),
        (RoleScope::Personal, Some(_)) => Err(scope_issue(format!(
            "role {id} is personal and has no group"
        ))),
        _ => Ok(()),
    }
}

/// The role can be held by a card of `kind`.
pub(super) fn check_kind(definition: &RoleDefinition, kind: &str) -> Result<()> {
    match &definition.card_kind {
        Some(needed) if needed != kind => Err(Error::Conflict(format!(
            "role {} needs a card of kind {needed}, this one would be {kind}",
            definition.id
        ))),
        _ => Ok(()),
    }
}

impl PeopleRepo {
    /// Every role `person` holds allows a card of `kind`.
    pub(super) async fn check_kind_against_roles(
        &self,
        person: ObjectId,
        kind: &str,
    ) -> Result<()> {
        for held in self.roles_of(person).await? {
            if let Some(definition) = self.role(&held.role).await? {
                check_kind(&definition, kind)?;
            }
        }
        Ok(())
    }
}
