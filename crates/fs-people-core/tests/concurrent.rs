//! Writes through the traits at the same time, over a backend that yields at every step: none is lost.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use common::{ME, person};
use fs_people_core::{Card, People, PeopleRepo, RoleBook};
use fs_objects::traits::{ObjectQuery, ObjectRepository};
use fs_objects::{Activity, LogEntry, ObjectId, Query, Store, StoredObject, TursoStore};
use serde_json::Map;
use tokio::task::yield_now;

/// The real store, with the pauses a remote one has.
struct Yielding(Arc<dyn Store>);

#[async_trait]
impl ObjectRepository for Yielding {
    async fn put(&self, object: StoredObject, actor: &str) -> fs_objects::Result<Activity> {
        yield_now().await;
        Store::put(&*self.0, object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> fs_objects::Result<Vec<Activity>> {
        yield_now().await;
        Store::put_and_log(&*self.0, object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> fs_objects::Result<Option<StoredObject>> {
        yield_now().await;
        Store::get(&*self.0, id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> fs_objects::Result<Activity> {
        yield_now().await;
        Store::delete(&*self.0, id, actor).await
    }
}

#[async_trait]
impl ObjectQuery for Yielding {
    async fn query(&self, query: &Query) -> fs_objects::Result<Vec<StoredObject>> {
        yield_now().await;
        Store::query(&*self.0, query).await
    }
    async fn count(&self, query: &Query) -> fs_objects::Result<usize> {
        yield_now().await;
        Store::count(&*self.0, query).await
    }
}

async fn repo() -> Arc<PeopleRepo> {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let backend = Arc::new(Yielding(store));
    Arc::new(PeopleRepo::with_backend(backend.clone(), backend, ME))
}

#[tokio::test(flavor = "current_thread")]
async fn members_added_at_once_all_stay() {
    let r = repo().await;
    let group = People::create_group(&*r, "All").await.unwrap().envelope.id;
    let mut members = vec![];
    for i in 0..10 {
        let card = person(&format!("M{i}"), "m@x.org");
        members.push(People::create(&*r, card).await.unwrap().envelope.id);
    }
    let mut calls = tokio::task::JoinSet::new();
    for member in members {
        let r = r.clone();
        calls.spawn(async move { People::add_member(&*r, group, member).await });
    }
    while let Some(done) = calls.join_next().await {
        done.unwrap().unwrap();
    }
    assert_eq!(People::members(&*r, group).await.unwrap().len(), 10);
}

#[tokio::test(flavor = "current_thread")]
async fn a_uid_is_taken_once_and_a_role_held_once() {
    let r = repo().await;
    let mut calls = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let r = r.clone();
        let mut card = Card::individual("Ada");
        card.uid = "one".into();
        calls.spawn(async move { People::create(&*r, card).await });
    }
    let mut ids = vec![];
    while let Some(done) = calls.join_next().await {
        if let Ok(stored) = done.unwrap() {
            ids.push(stored.envelope.id);
        }
    }
    assert_eq!(ids.len(), 1);

    let mut calls = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let r = r.clone();
        let id = ids[0];
        calls.spawn(async move {
            RoleBook::assign_role(&*r, id, "member", Some("https://x/g"), Map::new()).await
        });
    }
    while let Some(done) = calls.join_next().await {
        done.unwrap().unwrap();
    }
    assert_eq!(RoleBook::roles_of(&*r, ids[0]).await.unwrap().len(), 1);
}
