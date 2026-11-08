//! Errors of the core as RPC errors.
//!
//! Pattern: Adapter — each error the core can name becomes a typed RFC 9457 problem, so a consumer
//! can tell "not found" from "not possible" without reading a message. A request that is malformed
//! never reaches the core (`invalid_request` in `wire`); what fails on data already stored is not the
//! caller's fault and says so.

use std::future::Future;

use fs_interfaces::rpc::{Problem, failed, run_to_end, too_large, unsupported};
use fs_objects::Error as StoreError;
use fs_people_core::Error;

use crate::facts::with_facts;

const NOT_FOUND: &str = "https://freesynergy.net/problems/people/not-found";
const CONFLICT: &str = "https://freesynergy.net/problems/people/conflict";
const INVALID: &str = "https://freesynergy.net/problems/people/invalid";
const SENSITIVE: &str = "https://freesynergy.net/problems/people/sensitive";
const ENCRYPTION_REQUIRED: &str = "https://freesynergy.net/problems/people/encryption-required";
const PARTLY_DELETED: &str = "https://freesynergy.net/problems/people/partly-deleted";
const SENSITIVE_LINK: &str = "https://freesynergy.net/problems/people/sensitive-link";
const SENSITIVE_MEMBERS: &str = "https://freesynergy.net/problems/people/sensitive-members";
const STORED_DATA: &str = "https://freesynergy.net/problems/people/stored-data";

/// A core error as an RPC error.
#[allow(clippy::needless_pass_by_value)] // used as `map_err(rpc_error)`
pub(crate) fn rpc_error(e: Error) -> capnp::Error {
    let text = e.to_string();
    let problem = |kind: &str, title: &str, status: u16| {
        with_facts(Problem::new(kind, title, text.clone(), status), &e).into_error()
    };
    match &e {
        Error::Unsupported(not_possible) => unsupported(not_possible),
        Error::TooLarge(_) => too_large(text.clone()),
        Error::NotFound(_)
        | Error::ListNotFound(_)
        | Error::UnknownRole(_)
        | Error::Store(StoreError::NotFound(_)) => problem(NOT_FOUND, "Not found", 404),
        Error::Conflict(_)
        | Error::Exists(_)
        | Error::Changed(_)
        | Error::Store(StoreError::Conflict(_) | StoreError::OriginalImmutable(_)) => {
            problem(CONFLICT, "Conflict", 409)
        }
        // Only text from the request reaches `Json`; stored data arrives as `Stored`.
        Error::Invalid(_)
        | Error::InexactNumber(_)
        | Error::Json(_)
        | Error::Store(StoreError::Invalid(_)) => {
            problem(INVALID, "Against the rules of the contract", 422)
        }
        Error::NotReleased(..) => {
            problem(SENSITIVE, "A sensitive card needs an explicit release", 403)
        }
        Error::SensitiveMembers(..) => problem(
            SENSITIVE_MEMBERS,
            "Sensitive members are not taken along",
            403,
        ),
        Error::Sensitive(_) => problem(SENSITIVE_LINK, "Sensitive cards are not linked", 403),
        Error::Store(StoreError::EncryptionRequired(_)) => problem(
            ENCRYPTION_REQUIRED,
            "This store does not take sensitive cards",
            409,
        ),
        // An import reports its bad vCards itself, so this is a stored card that has no vCard form.
        Error::Stored(_) | Error::VCard(_) | Error::Store(StoreError::Serde(_)) => {
            problem(STORED_DATA, "Stored data cannot be read", 500)
        }
        // The person went and something of them stayed: a consumer has to be able to tell, and to
        // call again, without reading a message.
        Error::PartlyDeleted(..) => problem(PARTLY_DELETED, "Deleted, not everything of it", 500),
        Error::Store(StoreError::Key(_) | StoreError::Backend(_)) => failed(text.clone()),
    }
}

/// Run a write of the provider to its end, also if the caller goes away meanwhile: dropped halfway
/// it would leave a person half deleted or an import half done, with nobody told.
pub(crate) async fn finish<T: 'static>(
    work: impl Future<Output = fs_people_core::Result<T>> + 'static,
) -> capnp::Result<T> {
    run_to_end(work).await?.map_err(rpc_error)
}
