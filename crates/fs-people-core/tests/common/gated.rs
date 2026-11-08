//! The real store, holding the first write at a gate once armed, and counting every call.
//!
//! Pattern: Decorator.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use fs_objects::traits::{ObjectQuery, ObjectRepository};
use fs_objects::{Activity, LogEntry, ObjectId, Query, Store, StoredObject};
use tokio::sync::{Notify, Semaphore};

pub struct Gated {
    store: Arc<dyn Store>,
    pub calls: AtomicUsize,
    pub gate: Semaphore,
    pub armed: AtomicUsize,
    pub arrived: Notify,
}

impl Gated {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self {
            store,
            calls: AtomicUsize::new(0),
            gate: Semaphore::new(0),
            armed: AtomicUsize::new(0),
            arrived: Notify::new(),
        }
    }
    fn pass(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }
    async fn pass_write(&self) {
        self.pass();
        if self.armed.swap(0, Ordering::SeqCst) == 1 {
            self.arrived.notify_one();
            drop(self.gate.acquire().await.unwrap());
        }
    }
}

#[async_trait]
impl ObjectRepository for Gated {
    async fn put(&self, object: StoredObject, actor: &str) -> fs_objects::Result<Activity> {
        self.pass_write().await;
        Store::put(&*self.store, object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> fs_objects::Result<Vec<Activity>> {
        self.pass_write().await;
        Store::put_and_log(&*self.store, object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> fs_objects::Result<Option<StoredObject>> {
        self.pass();
        Store::get(&*self.store, id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> fs_objects::Result<Activity> {
        self.pass_write().await;
        Store::delete(&*self.store, id, actor).await
    }
}

#[async_trait]
impl ObjectQuery for Gated {
    async fn query(&self, query: &Query) -> fs_objects::Result<Vec<StoredObject>> {
        self.pass();
        Store::query(&*self.store, query).await
    }
    async fn count(&self, query: &Query) -> fs_objects::Result<usize> {
        self.pass();
        Store::count(&*self.store, query).await
    }
}
