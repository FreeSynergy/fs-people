//! The table of problems, read back: every type of the server and the variant it must become.
//!
//! Pattern: none — a test table (input, expected variant) for the types the real server cannot be made to send in
//! a test (stored data, too large, partly deleted); the others are covered over the real socket.

use fs_interfaces::rpc::Problem;
use fs_people_core::Error;
use fs_objects::{Error as StoreError, ObjectId};

use crate::problems::{Subject, error_of};
use fs_interfaces::rpc::client::problems::PROBLEM_BASE as BASE;

fn said(kind: &str, status: u16, detail: &str, subject: &Subject) -> Error {
    titled(kind, "t", status, detail, subject)
}

fn titled(kind: &str, title: &str, status: u16, detail: &str, subject: &Subject) -> Error {
    let problem = Problem::new(format!("{BASE}{kind}"), title, detail, status);
    error_of(&problem.into_error(), subject)
}

#[test]
fn the_rest_of_the_table_of_the_server_comes_back() {
    let id = ObjectId::new();
    let at = Subject::Person(id);
    let none = Subject::None;
    let e = said("people/stored-data", 500, "stored data: broken", &none);
    assert!(matches!(&e, Error::Stored(t) if t == "broken"), "{e:?}");
    let e = said(
        "too-large",
        413,
        "the card has 99 bytes, at most 5 are kept",
        &none,
    );
    assert!(matches!(e, Error::TooLarge(99)), "{e:?}");
    let e = said(
        "people/sensitive-members",
        403,
        "sensitive members need an explicit release: a, b",
        &none,
    );
    assert!(
        matches!(&e, Error::SensitiveMembers(n) if n == &["a", "b"]),
        "{e:?}"
    );
    let e = said(
        "people/partly-deleted",
        500,
        "the links stayed; delete again to finish it",
        &at,
    );
    assert!(
        matches!(&e, Error::PartlyDeleted(i, t) if *i == id && t == "the links stayed"),
        "{e:?}"
    );
    let e = said("unsupported", 501, "person.find: not here", &none);
    assert!(
        matches!(&e, Error::Unsupported(u) if u.capability() == "person.find" && u.reason() == "not here"),
        "{e:?}"
    );
    let e = said(
        "people/not-found",
        404,
        "role x",
        &Subject::Role("x".into()),
    );
    assert!(matches!(&e, Error::UnknownRole(r) if r == "x"), "{e:?}");
    let e = said("invalid-request", 422, "invalid cursor", &none);
    assert!(matches!(e, Error::Store(StoreError::Invalid(_))), "{e:?}");
}

#[test]
fn every_text_comes_back_as_the_core_would_have_said_it() {
    let issues = "card violates RFC 9553: /uid: required; /version: must be 1.0";
    let e = said("people/invalid", 422, issues, &Subject::None);
    assert_eq!(e.to_string(), issues);
    let e = said(
        "people/sensitive",
        403,
        "urn:uuid:1: Ada is sensitive and needs an explicit release",
        &Subject::None,
    );
    assert_eq!(
        e.to_string(),
        "urn:uuid:1: Ada is sensitive and needs an explicit release"
    );
}

#[test]
fn an_invalid_text_is_not_wrapped_a_second_time() {
    let none = Subject::None;
    let text = "object store: invalid input: name is empty";
    let e = said("people/invalid", 422, text, &none);
    assert!(
        matches!(&e, Error::Store(StoreError::Invalid(t)) if t == "name is empty"),
        "{e:?}"
    );
    assert_eq!(e.to_string(), text);
}

#[test]
fn an_inexact_number_and_a_json_error_come_back_as_themselves() {
    let none = Subject::None;
    let text = "the number 1e999 cannot be kept exactly; write it as a string";
    let e = said("people/invalid", 422, text, &none);
    assert!(
        matches!(&e, Error::InexactNumber(n) if n == "1e999"),
        "{e:?}"
    );
    assert_eq!(e.to_string(), text);
    let text = "json: expected value at line 1 column 1";
    let e = said("people/invalid", 422, text, &none);
    assert!(matches!(e, Error::Json(_)), "{e:?}");
    assert_eq!(e.to_string(), text);
}

#[test]
fn the_two_meanings_of_sensitive_are_told_apart_by_the_type_alone() {
    let id = ObjectId::new();
    let none = Subject::None;
    let linked =
        format!("person {id} carries sensitive categories and must not be linked (ADR 0029)");
    let e = said("people/sensitive-link", 403, &linked, &none);
    assert!(matches!(e, Error::Sensitive(i) if i == id), "{e:?}");
    assert_eq!(e.to_string(), linked);
    // The same words under the other type are not read as a link: only the type decides.
    let e = said("people/sensitive", 403, &linked, &none);
    assert!(matches!(&e, Error::NotReleased(_)), "{e:?}");
    let text = "urn:uuid:1: Ada is sensitive and needs an explicit release";
    let e = said("people/sensitive", 403, text, &none);
    assert!(
        matches!(&e, Error::NotReleased(w) if w == "urn:uuid:1: Ada"),
        "{e:?}"
    );
}
