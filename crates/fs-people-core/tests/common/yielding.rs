//! The real store with the pauses a remote one has: every call yields first, so two writers meet.
//!
//! Pattern: Decorator.

use std::sync::Arc;

use async_trait::async_trait;
use fs_people_core::PeopleRepo;
use fs_store::traits::{ObjectQuery, ObjectRepository};
use fs_store::{Activity, LogEntry, ObjectId, Query, Store, StoredObject, TursoStore};
use tokio::task::yield_now;

/// The real store, with the pauses a remote one has.
pub struct Yielding(Arc<dyn Store>);

#[async_trait]
impl ObjectRepository for Yielding {
    async fn put(&self, object: StoredObject, actor: &str) -> fs_store::Result<Activity> {
        yield_now().await;
        Store::put(&*self.0, object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> fs_store::Result<Vec<Activity>> {
        yield_now().await;
        Store::put_and_log(&*self.0, object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> fs_store::Result<Option<StoredObject>> {
        yield_now().await;
        Store::get(&*self.0, id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> fs_store::Result<Activity> {
        yield_now().await;
        Store::delete(&*self.0, id, actor).await
    }
}

#[async_trait]
impl ObjectQuery for Yielding {
    async fn query(&self, query: &Query) -> fs_store::Result<Vec<StoredObject>> {
        yield_now().await;
        Store::query(&*self.0, query).await
    }
    async fn count(&self, query: &Query) -> fs_store::Result<usize> {
        yield_now().await;
        Store::count(&*self.0, query).await
    }
}

pub async fn yielding_repo() -> Arc<PeopleRepo> {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let backend = Arc::new(Yielding(store));
    Arc::new(PeopleRepo::with_backend(
        backend.clone(),
        backend,
        super::ME,
    ))
}
