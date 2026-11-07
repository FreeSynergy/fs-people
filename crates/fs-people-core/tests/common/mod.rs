//! Shared helpers of the integration tests.

#![allow(dead_code)] // each test file uses a different subset

use std::sync::Arc;

use fs_people_core::{Card, PeopleRepo};
use fs_objects::{Store, TursoStore};

pub mod yielding;

pub const ME: &str = "https://local.invalid/users/owner";
/// The demo cards; `FS_PEOPLE_DEMO` names a copy to read instead of the documentation tree.
pub const DEMO: &str = match option_env!("FS_PEOPLE_DEMO") {
    Some(copy) => copy,
    None => "/srv/FreeSynergy/fs-documentation/temp/demo-data/contacts.json",
};

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
