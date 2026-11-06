//! Errors of the core as RPC errors.
//!
//! Pattern: Adapter — each error the core can name becomes a typed RFC 9457 problem, so a consumer
//! can tell "not found" from "not possible" without reading a message.

use fs_interfaces::rpc::{Problem, failed, unsupported};
use fs_people_core::Error;
use fs_store::Error as StoreError;

const NOT_FOUND: &str = "https://freesynergy.net/problems/people/not-found";
const CONFLICT: &str = "https://freesynergy.net/problems/people/conflict";
const INVALID: &str = "https://freesynergy.net/problems/people/invalid";
const SENSITIVE: &str = "https://freesynergy.net/problems/people/sensitive";
const ENCRYPTION_REQUIRED: &str = "https://freesynergy.net/problems/people/encryption-required";
const TOO_LARGE: &str = "https://freesynergy.net/problems/people/too-large";

/// A core error as an RPC error.
pub(crate) fn rpc_error(e: Error) -> capnp::Error {
    let text = e.to_string();
    match e {
        Error::Unsupported(not_possible) => unsupported(&not_possible),
        Error::NotFound(_) | Error::Store(StoreError::NotFound(_)) => {
            Problem::new(NOT_FOUND, "Not found", text, 404).into_error()
        }
        Error::Conflict(_)
        | Error::Store(StoreError::Conflict(_) | StoreError::OriginalImmutable(_)) => {
            Problem::new(CONFLICT, "Conflict", text, 409).into_error()
        }
        Error::Invalid(_)
        | Error::VCard(_)
        | Error::Json(_)
        | Error::Store(StoreError::Invalid(_)) => invalid(text),
        Error::Sensitive(_) => {
            Problem::new(SENSITIVE, "Sensitive cards are not linked", text, 403).into_error()
        }
        Error::Store(StoreError::EncryptionRequired(_)) => Problem::new(
            ENCRYPTION_REQUIRED,
            "This store does not take sensitive cards",
            text,
            409,
        )
        .into_error(),
        Error::Store(_) => failed(text),
    }
}

/// The request itself is not acceptable.
pub(crate) fn invalid(detail: impl Into<String>) -> capnp::Error {
    Problem::new(INVALID, "Invalid request", detail, 422).into_error()
}

/// An answer without a cursor that would not fit into one message. Cutting it would hide people
/// without saying so, and sending it would end the whole connection.
pub(crate) fn too_large(detail: impl Into<String>) -> capnp::Error {
    Problem::new(
        TOO_LARGE,
        "The answer does not fit one message",
        detail,
        413,
    )
    .into_error()
}
