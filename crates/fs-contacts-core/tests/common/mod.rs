//! Shared helpers of the integration tests.

#![allow(dead_code)] // each test file uses a different subset

use std::sync::Arc;

use fs_contacts_core::{Card, ContactRepo};
use fs_store::{Store, TursoStore};

pub const ME: &str = "https://local.invalid/users/owner";
pub const DEMO: &str = "/srv/FreeSynergy/fs-documentation/temp/demo-data/contacts.json";

/// A repository over a fresh in-memory store.
pub async fn repo() -> ContactRepo {
    setup().await.0
}

/// The repository and a handle to its store (for undo, log and count checks).
pub async fn setup() -> (ContactRepo, Arc<dyn Store>) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    (ContactRepo::new(store.clone(), ME), store)
}

/// A person card with one e-mail address.
pub fn person(name: &str, email: &str) -> Card {
    let mut c = Card::individual(name);
    c.add_email(email);
    c
}
