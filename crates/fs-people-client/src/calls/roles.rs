//! Calls of `Roles`.
//!
//! Pattern: Adapter — one request in, one reply read back as values of the core.

use fs_interfaces::people_capnp::roles::Client;
use fs_interfaces::rpc::write_object_id;
use fs_people_core::{Result, RoleAssignment, RoleDefinition};
use fs_objects::ObjectId;

use crate::problems::{Subject, error_of};
use crate::read::{assignment, definition, unreadable};

/// `get`: `None` when the server says `found = false`.
pub(crate) async fn get(c: &Client, id: String) -> Result<Option<RoleDefinition>> {
    let mut req = c.get_request();
    req.get().set_id(id.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Role(id)))?;
    let r = answer.get().map_err(unreadable)?;
    if !r.get_found() {
        return Ok(None);
    }
    definition(r.get_definition().map_err(unreadable)?).map(Some)
}

/// `rolesOf`.
pub(crate) async fn roles_of(c: &Client, person: ObjectId) -> Result<Vec<RoleAssignment>> {
    let mut req = c.roles_of_request();
    write_object_id(req.get().init_person(), person);
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::Person(person)))?;
    answer
        .get()
        .and_then(|r| r.get_items())
        .map_err(unreadable)?
        .iter()
        .map(assignment)
        .collect()
}
