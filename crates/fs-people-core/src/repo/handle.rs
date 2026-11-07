//! [`PeopleRepo`]: the store-backed implementation of `People`.
//!
//! Pattern: Repository + Facade — one small type hides the object store, the envelope rules, the
//! formats and the matching strategy behind the `People` contract. Its collaborators are
//! injected as traits (`ObjectRepository`, `ObjectQuery`, `MatchStrategy`), so each can be swapped.

use std::sync::Arc;

use fs_objects::traits::{ObjectQuery, ObjectRepository};
use fs_objects::{ObjectId, Store, StoreInfo};
use tokio::sync::Mutex;

use super::backend::StoreAdapter;
use crate::feed::Registry;
use crate::matching::{MatchStrategy, SharedContactPoint};
use crate::roles::{RoleDirectory, RoleRegistry};

/// People over an object store. Cheap to clone; clones share their subscribers.
#[derive(Clone)]
pub struct PeopleRepo {
    pub(super) objects: Arc<dyn ObjectRepository>,
    pub(super) reads: Arc<dyn ObjectQuery>,
    pub(super) matcher: Arc<dyn MatchStrategy>,
    pub(super) registry: Arc<dyn RoleDirectory>,
    pub(super) info: Option<Arc<dyn StoreInfo>>,
    pub(super) feed: Arc<Registry>,
    /// One write at a time through the traits; clones share it.
    pub(super) writes: Arc<Mutex<()>>,
    pub(super) actor: String,
    pub(super) instance: String,
}

impl PeopleRepo {
    /// People over a whole [`Store`]. `actor` is the IRI of the acting person / program
    /// (written to the log).
    pub fn new(store: Arc<dyn Store>, actor: impl Into<String>) -> Self {
        let adapter = Arc::new(StoreAdapter(store));
        Self::with_backend(adapter.clone(), adapter.clone(), actor).with_info(adapter)
    }

    /// People over the narrow store traits only (a remote or fake backend can supply them).
    pub fn with_backend(
        objects: Arc<dyn ObjectRepository>,
        reads: Arc<dyn ObjectQuery>,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            objects,
            reads,
            matcher: Arc::new(SharedContactPoint),
            registry: Arc::new(RoleRegistry::known()),
            info: None,
            feed: Arc::default(),
            writes: Arc::default(),
            actor: actor.into(),
            instance: "local.invalid".into(),
        }
    }

    /// Say whose store this is: [`People::identity`](crate::People::identity) answers from it.
    #[must_use]
    pub fn with_info(mut self, info: Arc<dyn StoreInfo>) -> Self {
        self.info = Some(info);
        self
    }

    /// Instance domain for external IRIs `https://<instance>/people/<id>` (ADR 0027).
    #[must_use]
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = instance.into();
        self
    }

    /// Use another same-person strategy (default: [`SharedContactPoint`]).
    #[must_use]
    pub fn with_match_strategy(mut self, matcher: Arc<dyn MatchStrategy>) -> Self {
        self.matcher = matcher;
        self
    }

    /// Use this directory of roles instead of the shipped ones (default: [`RoleRegistry::known`]).
    #[must_use]
    pub fn with_roles(mut self, directory: impl RoleDirectory + 'static) -> Self {
        self.registry = Arc::new(directory);
        self
    }

    pub(super) fn iri(&self, id: ObjectId) -> String {
        format!("https://{}/people/{}", self.instance, id)
    }
}
