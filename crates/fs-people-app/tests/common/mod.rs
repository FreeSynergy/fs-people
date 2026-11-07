//! Shared helpers of the tests: the real core over an in-memory store, behind the real adapter.
//!
//! Pattern: Facade over the set-up. Nothing here pretends to be the core.
#![allow(dead_code)] // each test file uses a different subset

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_components_workbenches::people::{PeopleLabels, PeopleSource, PeopleState, Variant};
use fs_people_app::PeopleAdapter;
use fs_people_core::{Card, PeopleRepo};
use fs_objects::{ObjectId, Store, TursoStore};
use tokio::runtime::Runtime;

pub const ME: &str = "https://local.invalid/users/owner";

/// A runtime, a repository over a store, and the adapter over it.
pub struct World {
    pub runtime: Runtime,
    pub repo: Arc<PeopleRepo>,
    pub adapter: Arc<PeopleAdapter>,
}

impl World {
    /// Over a fresh in-memory store.
    pub fn in_memory() -> Self {
        let runtime = Runtime::new().unwrap();
        let store = runtime.block_on(TursoStore::open_in_memory()).unwrap();
        Self::over(runtime, Arc::new(store))
    }

    /// Over a plain store file, which refuses sensitive cards.
    pub fn in_file(path: &Path) -> Self {
        let runtime = Runtime::new().unwrap();
        let store = runtime.block_on(TursoStore::open(path)).unwrap();
        Self::over(runtime, Arc::new(store))
    }

    fn over(runtime: Runtime, store: Arc<dyn Store>) -> Self {
        let repo = Arc::new(PeopleRepo::new(store, ME));
        let adapter = Arc::new(PeopleAdapter::new(
            repo.clone(),
            repo.clone(),
            runtime.handle().clone(),
        ));
        Self {
            runtime,
            repo,
            adapter,
        }
    }

    /// Store `card` through the core; returns the row key.
    pub fn add(&self, card: Card) -> String {
        let stored = self.runtime.block_on(self.repo.create(card)).unwrap();
        stored.envelope.id.to_string()
    }

    /// The card as stored now.
    pub fn card(&self, key: &str) -> Card {
        let id: ObjectId = key.parse().unwrap();
        self.runtime
            .block_on(self.repo.get_card(id))
            .unwrap()
            .unwrap()
    }

    /// The state machine over the real core.
    pub fn state(&self) -> PeopleState {
        let source: Arc<dyn PeopleSource> = self.adapter.clone();
        PeopleState::new(source, fs_people_app::labels(), Variant::Full)
    }

    /// The labels, for a test that wants to change them.
    pub fn labels() -> PeopleLabels {
        fs_people_app::labels()
    }
}

/// A person card with one e-mail address.
pub fn person(name: &str, email: &str) -> Card {
    let mut card = Card::individual(name);
    card.add_email(email);
    card
}

/// A folder for the files of one test, removed when the test ends (also when it fails).
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("fs-people-app-{}", ObjectId::new()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
