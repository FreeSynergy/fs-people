//! `RevisedPeople`: a person together with the revision the server gives it.
//!
//! Pattern: Strategy (a narrow trait beside `People`, so a frontend depends on `dyn RevisedPeople`
//! and not on [`RpcPeople`](crate::RpcPeople)). Why a trait of its own: `StoredObject` has no place
//! for the revision, and it cannot be derived from the object read here (the wire keeps time in
//! milliseconds, the store in microseconds), so the number from `Person.revision` is handed on as it
//! came. It is for [`People::update_checked`](fs_people_core::People::update_checked); compare it
//! for equality only. It lives in the client for now and moves up to `fs-people-core` with a second
//! user (`DESIGN.md`).

use async_trait::async_trait;
use fs_people_core::{Result, Revision};
use fs_store::{ObjectId, StoredObject};

/// A person as read, and its revision.
#[derive(Debug, Clone)]
pub struct Revised {
    /// The person.
    pub object: StoredObject,
    /// What the server says the person is now: the `expected` of `update_checked`.
    pub revision: Revision,
}

/// Reading a person with its revision.
#[async_trait]
pub trait RevisedPeople: Send + Sync {
    /// The person and its revision; `None` when there is no such person.
    async fn get_revised(&self, id: ObjectId) -> Result<Option<Revised>>;
}
