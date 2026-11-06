//! Errors of the core as RPC errors.
//!
//! Pattern: Adapter — each error the core can name becomes a typed RFC 9457 problem, so a consumer
//! can tell "not found" from "not possible" without reading a message. A request that is malformed
//! never reaches the core (`invalid_request` in `wire`); what fails on data already stored is not the
//! caller's fault and says so.

use std::future::Future;

use fs_interfaces::rpc::{Problem, failed, run_to_end, too_large, unsupported};
use fs_people_core::Error;
use fs_store::Error as StoreError;

const NOT_FOUND: &str = "https://freesynergy.net/problems/people/not-found";
const CONFLICT: &str = "https://freesynergy.net/problems/people/conflict";
const INVALID: &str = "https://freesynergy.net/problems/people/invalid";
const SENSITIVE: &str = "https://freesynergy.net/problems/people/sensitive";
const ENCRYPTION_REQUIRED: &str = "https://freesynergy.net/problems/people/encryption-required";
const PARTLY_DELETED: &str = "https://freesynergy.net/problems/people/partly-deleted";
const SENSITIVE_MEMBERS: &str = "https://freesynergy.net/problems/people/sensitive-members";
const STORED_DATA: &str = "https://freesynergy.net/problems/people/stored-data";

/// A core error as an RPC error.
pub(crate) fn rpc_error(e: Error) -> capnp::Error {
    let text = e.to_string();
    match e {
        Error::Unsupported(not_possible) => unsupported(&not_possible),
        Error::TooLarge(_) => too_large(text),
        Error::NotFound(_) | Error::UnknownRole(_) | Error::Store(StoreError::NotFound(_)) => {
            Problem::new(NOT_FOUND, "Not found", text, 404).into_error()
        }
        Error::Conflict(_)
        | Error::Store(StoreError::Conflict(_) | StoreError::OriginalImmutable(_)) => {
            Problem::new(CONFLICT, "Conflict", text, 409).into_error()
        }
        // Only text from the request reaches `Json`; stored data arrives as `Stored`.
        Error::Invalid(_)
        | Error::InexactNumber(_)
        | Error::Json(_)
        | Error::Store(StoreError::Invalid(_)) => {
            Problem::new(INVALID, "Against the rules of the contract", text, 422).into_error()
        }
        Error::SensitiveMembers(_) => Problem::new(
            SENSITIVE_MEMBERS,
            "Sensitive members are not taken along",
            text,
            403,
        )
        .into_error(),
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
        // An import reports its bad vCards itself, so this is a stored card that has no vCard form.
        Error::Stored(_) | Error::VCard(_) | Error::Store(StoreError::Serde(_)) => {
            Problem::new(STORED_DATA, "Stored data cannot be read", text, 500).into_error()
        }
        // The person went and something of them stayed: a consumer has to be able to tell, and to
        // call again, without reading a message.
        Error::PartlyDeleted(..) => {
            Problem::new(PARTLY_DELETED, "Deleted, not everything of it", text, 500).into_error()
        }
        Error::Store(StoreError::Key(_) | StoreError::Backend(_)) => failed(text),
    }
}

/// Run a write of the provider to its end, also if the caller goes away meanwhile: dropped halfway
/// it would leave a person half deleted or an import half done, with nobody told.
pub(crate) async fn finish<T: 'static>(
    work: impl Future<Output = fs_people_core::Result<T>> + 'static,
) -> capnp::Result<T> {
    run_to_end(work).await?.map_err(rpc_error)
}
