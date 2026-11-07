//! Finding 14 of the review: a failed delete must not leave a person half deleted.
//!
//! Pattern: Decorator around the real store, whose deletes fail on request.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use common::{ME, person};
use fs_people_core::{Error, PeopleRepo};
use fs_objects::traits::{ObjectQuery, ObjectRepository};
use fs_objects::{Activity, LogEntry, ObjectId, Query, Store, StoredObject, TursoStore};

/// A real store whose deletes fail on request.
struct Failing {
    store: Arc<dyn Store>,
    fail_cards: AtomicBool,
    fail_others: AtomicBool,
}

#[async_trait]
impl ObjectRepository for Failing {
    async fn put(&self, object: StoredObject, actor: &str) -> fs_objects::Result<Activity> {
        Store::put(&*self.store, object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> fs_objects::Result<Vec<Activity>> {
        Store::put_and_log(&*self.store, object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> fs_objects::Result<Option<StoredObject>> {
        Store::get(&*self.store, id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> fs_objects::Result<Activity> {
        let is_card = Store::get(&*self.store, id)
            .await?
            .is_some_and(|o| o.envelope.object_type == "Profile");
        let flag = if is_card {
            &self.fail_cards
        } else {
            &self.fail_others
        };
        if flag.load(Ordering::Relaxed) {
            return Err(fs_objects::Error::Invalid("the disk is full".into()));
        }
        Store::delete(&*self.store, id, actor).await
    }
}

#[async_trait]
impl ObjectQuery for Failing {
    async fn query(&self, query: &Query) -> fs_objects::Result<Vec<StoredObject>> {
        Store::query(&*self.store, query).await
    }
    async fn count(&self, query: &Query) -> fs_objects::Result<usize> {
        Store::count(&*self.store, query).await
    }
}

async fn with_failing_store() -> (PeopleRepo, Arc<Failing>) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let failing = Arc::new(Failing {
        store,
        fail_cards: AtomicBool::new(false),
        fail_others: AtomicBool::new(false),
    });
    let repo = PeopleRepo::with_backend(failing.clone(), failing.clone(), ME);
    (repo, failing)
}

#[tokio::test]
async fn item_14_a_refused_delete_of_the_card_leaves_links_and_roles_in_place() {
    let (r, failing) = with_failing_store().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let b = r
        .create(person("Bob", "b@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.confirm_same_person(a, b).await.unwrap();
    r.mark_ai(a).await.unwrap();
    failing.fail_cards.store(true, Ordering::Relaxed);
    let said = r.delete(a).await.unwrap_err();
    assert!(
        matches!(said, Error::Store(_)),
        "nothing happened, so no partial error: {said}"
    );
    assert!(r.get(a).await.unwrap().is_some());
    assert_eq!(
        r.linked_to(a).await.unwrap(),
        [b],
        "the link is still there"
    );
    assert!(r.is_ai(a).await.unwrap(), "the role is still there");
}

#[tokio::test]
async fn item_14_a_failed_clean_up_is_said_and_a_second_delete_finishes_it() {
    let (r, failing) = with_failing_store().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let b = r
        .create(person("Bob", "b@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.confirm_same_person(a, b).await.unwrap();
    failing.fail_others.store(true, Ordering::Relaxed);
    let said = r.delete(a).await.unwrap_err();
    assert!(
        matches!(said, Error::PartlyDeleted(id, _) if id == a),
        "{said}"
    );
    assert!(said.to_string().contains("the person is deleted"), "{said}");
    assert!(r.get(a).await.unwrap().is_none());
    failing.fail_others.store(false, Ordering::Relaxed);
    r.delete(a).await.unwrap();
    assert!(r.linked_to(b).await.unwrap().is_empty());
    assert!(
        matches!(r.delete(a).await, Err(Error::NotFound(_))),
        "nothing left to do"
    );
}

#[tokio::test]
async fn a_failed_card_delete_after_a_list_changed_says_so() {
    let (r, failing) = with_failing_store().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let list = r.create_group("L").await.unwrap().envelope.id;
    r.add_member(list, a).await.unwrap();
    // The list is a card too; only the delete of the person can fail here.
    failing.fail_cards.store(true, Ordering::Relaxed);
    let said = r.delete(a).await.unwrap_err();
    assert!(
        matches!(said, Error::PartlyDeleted(id, _) if id == a),
        "{said}"
    );
    assert!(said.to_string().contains("off 1 lists"), "{said}");
    assert!(said.to_string().contains("delete again"), "{said}");
    assert!(r.get(a).await.unwrap().is_some());
    failing.fail_cards.store(false, Ordering::Relaxed);
    r.delete(a).await.unwrap();
    assert!(r.get(a).await.unwrap().is_none());
}
