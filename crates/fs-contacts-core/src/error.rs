//! Error type of the contacts core.

use fs_store::ObjectId;

/// One validation problem, with a JSON-pointer-like path into the card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("object store: {0}")]
    Store(#[from] fs_store::Error),
    #[error("card violates RFC 9553: {}", .0.iter().map(|i| i.to_string()).collect::<Vec<_>>().join("; "))]
    Invalid(Vec<Issue>),
    #[error("contact {0} not found")]
    NotFound(ObjectId),
    #[error("vCard: {0}")]
    VCard(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// ADR 0029: sensitive cards are never linked.
    #[error("contact {0} carries sensitive categories and must not be linked (ADR 0029)")]
    Sensitive(ObjectId),
    #[error("{0}")]
    Conflict(String),
}
