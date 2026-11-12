//! Server problems and persons in the core's terms.
//!
//! Pattern: Adapter — wire values converted into the core's errors and stored objects.

use std::sync::PoisonError;

use fs_interfaces::Unsupported;
use fs_interfaces::people_capnp::person;
use fs_interfaces::rpc::client::connection_lost;
use fs_interfaces::rpc::{Problem, read_envelope};
use fs_objects::{ObjectId, StoredObject};
use fs_people_core::Error;

use super::call::Revisions;
use super::limits::LOST;

/// What the server said, in the core's terms. The server's own words are kept.
pub(super) fn problem(error: &capnp::Error, id: Option<ObjectId>) -> Error {
    // The connection ended under the call (the server went away, e.g. replaced by another window): not stored data.
    if connection_lost(error) {
        return Error::Unreachable(LOST.into());
    }
    match Problem::from_error(error) {
        Some(p) if p.is_unsupported() => Error::Unsupported(Unsupported::new("people", p.detail)),
        Some(p) if p.status == 404 => id.map_or(Error::Conflict(p.detail), Error::NotFound),
        // The server's compare-and-set says `conflict:` and names no revision.
        Some(p) if p.detail.starts_with("conflict:") && id.is_some() => {
            id.map_or(Error::Conflict(p.detail), Error::Changed)
        }
        Some(p) => Error::Conflict(p.detail),
        None => Error::Stored(error.to_string()),
    }
}

pub(super) fn stored(revisions: &Revisions, p: person::Reader<'_>) -> capnp::Result<StoredObject> {
    let envelope = read_envelope(p.get_envelope()?)?;
    let content = serde_json::from_str(&p.get_jscontact()?.to_string()?)
        .map_err(|e| capnp::Error::failed(e.to_string()))?;
    let id = envelope.id;
    revisions
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(id, p.get_revision());
    Ok(StoredObject::new(envelope, "application/json", content))
}
