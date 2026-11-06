//! Error type of the people core.
//!
//! Pattern: none (plain error enum via `thiserror`) — one `Error` for every public operation,
//! with `From` conversions for the store and JSON layers.

use fs_store::ObjectId;

/// The largest card as JSON: with its envelope it still has to fit one answer of the contract, and
/// small edits (relations, members) must not grow it past that.
pub const MAX_CARD_BYTES: usize = 16 * 1024 * 1024;

/// One validation problem, with a JSON-pointer-like path into the card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    /// JSON-pointer-like path, e.g. `/emails/e1`.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// Result of every fallible operation of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong in the people core.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The object store failed.
    #[error("object store: {0}")]
    Store(#[from] fs_store::Error),
    /// The card violates RFC 9553 (all problems listed).
    #[error("card violates RFC 9553: {}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))]
    Invalid(Vec<Issue>),
    /// No person with this id.
    #[error("person {0} not found")]
    NotFound(ObjectId),
    /// vCard text could not be read or written.
    #[error("vCard: {0}")]
    VCard(String),
    /// JSON could not be read or written.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// The card is gone, but links or roles that belonged to it could not all be removed. Deleting
    /// it again removes the rest.
    #[error(
        "the person is deleted, but links or roles that belonged to them remain ({1}); delete again to remove them"
    )]
    PartlyDeleted(ObjectId, String),
    /// ADR 0029: sensitive cards are never linked.
    #[error("person {0} carries sensitive categories and must not be linked (ADR 0029)")]
    Sensitive(ObjectId),
    /// The explicit "not possible" of a provider (ADR 0045).
    #[error(transparent)]
    Unsupported(#[from] fs_interfaces::Unsupported),
    /// Something already stored cannot be read any more; the request is not to blame.
    #[error("stored data: {0}")]
    Stored(String),
    /// A card too large to come back in one answer of the contract.
    #[error("the card has {0} bytes, at most {MAX_CARD_BYTES} are kept")]
    TooLarge(usize),
    /// The operation conflicts with the current state (duplicate uid, immutable field, ...).
    #[error("{0}")]
    Conflict(String),
}
