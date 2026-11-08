//! Taking over what a person shares, after a link (ADR 0058): the interface only.
//!
//! Pattern: Repository (interface) + Null Object — linking a profile to a person does not copy
//! anything from the person. What may be taken over depends on what the person released
//! (the sharing interface of the identity, not built yet), and taking it over is a step of its own
//! that the user asks for. This trait names the two calls so a provider can be written later; no
//! provider exists, every method answers `Unsupported`, and no capability id is offered for it.

use async_trait::async_trait;
use fs_objects::{ObjectId, StoredObject};

use crate::error::Result;
use crate::people::not_possible;

/// One detail the person has released for the user to take over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offered {
    /// Where it sits in the card, a JSON pointer such as `/emails/e1`.
    pub path: String,
    /// What it is, for the question to the user: `emails`, `phones`, `addresses`, ...
    pub label: String,
}

/// Adopting released details into the user's own profile of a linked person.
#[async_trait]
pub trait Adoption: Send + Sync {
    /// `profile.adopt`: what the person behind the linked profile released to this user. Nothing
    /// is copied by asking.
    async fn offered(&self, _link: ObjectId) -> Result<Vec<Offered>> {
        not_possible("profile.adopt")
    }
    /// `profile.adopt`: copy the chosen details (their `path`s) into the user's own profile and
    /// answer it. The user chose; the release of the person is what allows it.
    async fn adopt(&self, _link: ObjectId, _paths: &[String]) -> Result<StoredObject> {
        not_possible("profile.adopt")
    }
}
