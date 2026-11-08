//! The role calls against the server.
//!
//! Pattern: Adapter — each role question as a request on the server's roles capability.

use fs_interfaces::people_capnp::roles;
use fs_interfaces::rpc::{read_object_id, write_uuid};
use fs_objects::ObjectId;
use fs_people_core::{Result, RoleAssignment, RoleDefinition, RoleScope};

use super::convert::problem;

pub(super) async fn roles_of(roles: &roles::Client, id: ObjectId) -> Result<Vec<RoleAssignment>> {
    let mut request = roles.roles_of_request();
    write_uuid(request.get().init_person(), Some(id.as_uuid()));
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, Some(id)))?;
    let items = answer
        .get()
        .and_then(roles::roles_of_results::Reader::get_items)
        .map_err(|e| problem(&e, Some(id)))?;
    let mut held = Vec::new();
    for a in items {
        let (Ok(assignment), Ok(person)) = (
            a.get_id().and_then(read_object_id),
            a.get_person().and_then(read_object_id),
        ) else {
            continue;
        };
        let scope = a
            .get_scope()
            .and_then(|t| t.to_string().map_err(Into::into))
            .map_err(|e: capnp::Error| problem(&e, Some(id)))?;
        held.push(RoleAssignment {
            id: assignment,
            person,
            role: a
                .get_role()
                .and_then(|t| t.to_string().map_err(Into::into))
                .map_err(|e: capnp::Error| problem(&e, Some(id)))?,
            scope: (!scope.is_empty()).then_some(scope),
            details: serde_json::Map::new(),
        });
    }
    Ok(held)
}

/// What the window needs of a role: its id and its names.
pub(super) async fn role(roles: &roles::Client, id: &str) -> Result<Option<RoleDefinition>> {
    let mut request = roles.get_request();
    request.get().set_id(id);
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, None))?;
    let r = answer.get().map_err(|e| problem(&e, None))?;
    if !r.get_found() {
        return Ok(None);
    }
    let wire = r.get_definition().map_err(|e| problem(&e, None))?;
    let mut definition = RoleDefinition::new(id, "", "", RoleScope::default());
    definition.names.clear();
    for n in wire.get_names().map_err(|e| problem(&e, None))? {
        let (language, text) = (
            n.get_language()
                .and_then(|t| t.to_string().map_err(Into::into))
                .map_err(|e: capnp::Error| problem(&e, None))?,
            n.get_text()
                .and_then(|t| t.to_string().map_err(Into::into))
                .map_err(|e: capnp::Error| problem(&e, None))?,
        );
        definition.names.insert(language, text);
    }
    Ok(Some(definition))
}
