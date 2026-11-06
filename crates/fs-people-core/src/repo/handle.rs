//! [`PeopleRepo`]: the store-backed implementation of `People`.
//!
//! Pattern: Repository + Facade — one small type hides the object store, the envelope rules, the
//! formats and the matching strategy behind the `People` contract. Its collaborators are
//! injected as traits (`ObjectRepository`, `ObjectQuery`, `MatchStrategy`), so each can be swapped.

use std::sync::Arc;

use fs_store::traits::{ObjectQuery, ObjectRepository};
use fs_store::{ObjectId, Store};

use super::backend::StoreAdapter;
use crate::matching::{MatchStrategy, SharedContactPoint};

/// People over an object store. Cheap to clone.
#[derive(Clone)]
pub struct PeopleRepo {
    pub(super) objects: Arc<dyn ObjectRepository>,
    pub(super) reads: Arc<dyn ObjectQuery>,
    pub(super) matcher: Arc<dyn MatchStrategy>,
    pub(super) actor: String,
    pub(super) instance: String,
}

impl PeopleRepo {
    /// People over a whole [`Store`]. `actor` is the IRI of the acting person / program
    /// (written to the log).
    pub fn new(store: Arc<dyn Store>, actor: impl Into<String>) -> Self {
        let adapter = Arc::new(StoreAdapter(store));
        Self::with_backend(adapter.clone(), adapter, actor)
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
            actor: actor.into(),
            instance: "local.invalid".into(),
        }
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

    pub(super) fn iri(&self, id: ObjectId) -> String {
        format!("https://{}/people/{}", self.instance, id)
    }
}
