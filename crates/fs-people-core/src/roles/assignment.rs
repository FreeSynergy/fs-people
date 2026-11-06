//! A person holding a role.
//!
//! Pattern: Value object — read from an AS2 `Relationship` in the store. It is **not** part of the
//! card, so exporting a card never reveals a role (ADR 0029). Mirrors `RoleAssignment` of `people.capnp`.

use fs_store::ObjectId;
use serde_json::{Map, Value};

/// A person holding a role, personally or within a group.
#[derive(Clone, Debug, PartialEq)]
pub struct RoleAssignment {
    /// Id of the assignment object.
    pub id: ObjectId,
    /// The person.
    pub person: ObjectId,
    /// Id of the role.
    pub role: String,
    /// IRI of the group; `None` = personal.
    pub scope: Option<String>,
    /// Lossless extra data for roles owned by `fs-people`. Other owners hold their own fields.
    pub details: Map<String, Value>,
}
