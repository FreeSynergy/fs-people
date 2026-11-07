//! The problems of the server as the errors of the core.
//!
//! Pattern: Adapter — the inverse of `errors.rs` of `fs-people-server`: each RFC 9457 type becomes
//! the variant of `fs_people_core::Error` it came from, so the frontend's own reading of an error
//! (`reason()` of the adapter of the app) needs no change. The wire carries a type, a status and a
//! text, not the variant's fields; what can be read back from the text is, the rest comes from the
//! call (`Subject`).

use fs_interfaces::Unsupported;
use fs_interfaces::rpc::{Problem, is_unimplemented};
use fs_people_core::{Error, Issue};
use fs_store::{Error as StoreError, ObjectId};

pub(crate) const BASE: &str = "https://freesynergy.net/problems/";

/// What a call was about, for the variants that name it.
#[derive(Clone, Debug)]
pub(crate) enum Subject {
    /// Nothing in particular (a list, a search, an import).
    None,
    /// This person.
    Person(ObjectId),
    /// This role.
    Role(String),
}

/// The error of the core for what the server (or the wire) said.
pub(crate) fn error_of(e: &capnp::Error, subject: &Subject) -> Error {
    match Problem::from_error(e) {
        Some(problem) => from_problem(&problem, subject),
        None if is_unimplemented(e) => Unsupported::new(
            "people",
            "the server does not know this method of the contract",
        )
        .into(),
        None => Error::Store(StoreError::Backend(format!("people server: {e}"))),
    }
}

fn from_problem(p: &Problem, subject: &Subject) -> Error {
    let detail = p.detail.as_str();
    let kind = p.problem_type.strip_prefix(BASE).unwrap_or(&p.problem_type);
    match kind {
        "unsupported" => unsupported(detail),
        "people/not-found" => not_found(detail, subject),
        "people/conflict" => Error::Conflict(detail.to_string()),
        "people/invalid" => invalid(detail),
        "people/sensitive" => Error::NotReleased(
            detail
                .strip_suffix(" is sensitive and needs an explicit release")
                .unwrap_or(detail)
                .to_string(),
        ),
        "people/sensitive-members" => Error::SensitiveMembers(
            detail
                .split_once(": ")
                .map_or(detail, |(_, names)| names)
                .split(", ")
                .map(str::to_string)
                .collect(),
        ),
        "people/encryption-required" => encryption_required(detail, subject),
        "people/stored-data" => Error::Stored(
            detail
                .strip_prefix("stored data: ")
                .unwrap_or(detail)
                .to_string(),
        ),
        "people/partly-deleted" => match subject {
            Subject::Person(id) => Error::PartlyDeleted(
                *id,
                detail
                    .strip_suffix("; delete again to finish it")
                    .unwrap_or(detail)
                    .to_string(),
            ),
            _ => Error::Stored(detail.to_string()),
        },
        "too-large" => Error::TooLarge(first_number(detail)),
        "invalid-request" => Error::Store(StoreError::Invalid(detail.to_string())),
        _ => Error::Store(StoreError::Backend(format!(
            "people server ({}): {detail}",
            p.status
        ))),
    }
}

fn unsupported(detail: &str) -> Error {
    let (capability, reason) = detail.split_once(": ").unwrap_or(("people", detail));
    Unsupported::new(capability, reason).into()
}

fn not_found(detail: &str, subject: &Subject) -> Error {
    match subject {
        Subject::Person(id) => Error::NotFound(*id),
        Subject::Role(id) => Error::UnknownRole(id.clone()),
        Subject::None => Error::Stored(format!("the server named something missing: {detail}")),
    }
}

/// `card violates RFC 9553: <path>: <message>; ...` is read back into its issues; any other text
/// the server found invalid stays a text.
fn invalid(detail: &str) -> Error {
    let issues: Option<Vec<Issue>> =
        detail
            .strip_prefix("card violates RFC 9553: ")
            .and_then(|list| {
                list.split("; ")
                    .map(|one| {
                        let (path, message) = one.split_once(": ")?;
                        Some(Issue {
                            path: path.to_string(),
                            message: message.to_string(),
                        })
                    })
                    .collect()
            });
    match issues {
        Some(issues) => Error::Invalid(issues),
        None => Error::Store(StoreError::Invalid(detail.to_string())),
    }
}

/// The text names the object: `object store: object <id> is marked sensitive but ...`.
fn encryption_required(detail: &str, subject: &Subject) -> Error {
    let named = detail
        .split_whitespace()
        .find_map(|word| word.parse::<ObjectId>().ok());
    match (named, subject) {
        (Some(id), _) => Error::Store(StoreError::EncryptionRequired(id)),
        (None, Subject::Person(id)) => Error::Store(StoreError::EncryptionRequired(*id)),
        _ => Error::Conflict(detail.to_string()),
    }
}

/// The first run of digits, 0 if there is none.
fn first_number(text: &str) -> usize {
    let digits: String = text
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().unwrap_or(0)
}
