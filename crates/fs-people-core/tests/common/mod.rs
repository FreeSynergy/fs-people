//! Shared helpers of the integration tests.

#![allow(dead_code)] // each test file uses a different subset

use std::sync::Arc;

use fs_objects::{Store, TursoStore};
use fs_people_core::{Card, PeopleRepo};

pub mod yielding;

pub const ME: &str = "https://local.invalid/users/owner";
/// The demo cards: a copy of the documentation tree's `temp/demo-data/contacts.json`, kept here so the tests need nothing outside the repository.
pub const DEMO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/contacts.json");

/// A repository over a fresh in-memory store.
pub async fn repo() -> PeopleRepo {
    setup().await.0
}

/// The repository and a handle to its store (for undo, log and count checks).
pub async fn setup() -> (PeopleRepo, Arc<dyn Store>) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    (PeopleRepo::new(store.clone(), ME), store)
}

/// A person card with one e-mail address.
pub fn person(name: &str, email: &str) -> Card {
    let mut c = Card::individual(name);
    c.add_email(email);
    c
}
