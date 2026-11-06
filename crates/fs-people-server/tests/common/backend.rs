//! A real store behind a pause and a switch: what a remote backend does, and what a full disk does.
//!
//! Pattern: Decorator.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use fs_store::traits::{ObjectQuery, ObjectRepository};
use fs_store::{Activity, LogEntry, ObjectId, Query, Store, StoredObject};

pub struct Backend {
    pub store: Arc<dyn Store>,
    pub pause: Duration,
    /// Deleting anything but a card fails.
    pub fail_others: AtomicBool,
}

#[async_trait]
impl ObjectRepository for Backend {
    async fn put(&self, object: StoredObject, actor: &str) -> fs_store::Result<Activity> {
        tokio::time::sleep(self.pause).await;
        Store::put(&*self.store, object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> fs_store::Result<Vec<Activity>> {
        tokio::time::sleep(self.pause).await;
        Store::put_and_log(&*self.store, object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> fs_store::Result<Option<StoredObject>> {
        tokio::time::sleep(self.pause).await;
        Store::get(&*self.store, id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> fs_store::Result<Activity> {
        tokio::time::sleep(self.pause).await;
        let card = Store::get(&*self.store, id)
            .await?
            .is_some_and(|o| o.envelope.object_type == "Profile");
        if !card && self.fail_others.load(Ordering::Relaxed) {
            return Err(fs_store::Error::Backend("the disk is full".into()));
        }
        Store::delete(&*self.store, id, actor).await
    }
}

#[async_trait]
impl ObjectQuery for Backend {
    async fn query(&self, query: &Query) -> fs_store::Result<Vec<StoredObject>> {
        Store::query(&*self.store, query).await
    }
    async fn count(&self, query: &Query) -> fs_store::Result<usize> {
        Store::count(&*self.store, query).await
    }
}
