//! The calls the proxy hands to the worker thread.
//!
//! Pattern: Command — an enum of the operations, each with the channel for its answer.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use fs_objects::{ObjectId, StoredObject};
use fs_people_core::{ListFilter, Result, RoleAssignment, RoleDefinition};
use tokio::sync::oneshot;

pub(super) type Reply<T> = oneshot::Sender<Result<T>>;

/// The server's revision of each person as last read here. The adapter reads a person right before it
/// writes, so the entry is the revision of what it compared; a rebuilt object could not give it back.
pub(super) type Revisions = Arc<Mutex<HashMap<ObjectId, u64>>>;

pub(super) enum Call {
    Get(ObjectId, Reply<Option<StoredObject>>),
    List(ListFilter, Reply<Vec<StoredObject>>),
    Create(String, Reply<StoredObject>),
    Update(ObjectId, String, Reply<StoredObject>),
    UpdateIf(ObjectId, u64, String, Reply<StoredObject>),
    Delete(ObjectId, Reply<()>),
    RolesOf(ObjectId, Reply<Vec<RoleAssignment>>),
    Role(String, Reply<Option<RoleDefinition>>),
}
