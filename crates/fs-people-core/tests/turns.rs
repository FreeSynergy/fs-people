//! One write at a time: while one is under way, no other reaches the store. All of them, by a gate in
//! the backend, so a write without its turn fails here and not only when a race happens to show.
//!
//! Pattern: Decorator around the real store, which holds the first write at a gate.

mod common;
#[path = "common/gated.rs"]
mod gated;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use common::{ME, person};
use fs_people_core::{
    Card, JsContactFormat, People, PeopleRepo, RoleBook, RoleDefinition, RoleScope,
};
use fs_objects::{ObjectId, Store, TursoStore};
use gated::Gated;
use serde_json::Map;

const CLUB: &str = "https://x/groups/club";

#[derive(Clone)]
struct Fixture {
    r: Arc<PeopleRepo>,
    ada: ObjectId,
    bob: ObjectId,
    list: ObjectId,
    held: ObjectId,
    link: ObjectId,
}

type Write = Pin<Box<dyn Future<Output = ()> + Send>>;
type Second = fn(Fixture) -> Write;

/// The second write; only whether it waits counts, not how it ends.
macro_rules! write {
    ($name:literal, |$f:ident| $call:expr) => {
        (
            $name,
            (|$f: Fixture| -> Write {
                Box::pin(async move {
                    let _ = $call.await;
                })
            }) as fn(Fixture) -> Write,
        )
    };
}

fn writes() -> Vec<(&'static str, Second)> {
    vec![
        write!("relate", |f| People::relate(&*f.r, f.ada, "u", "friend")),
        write!("unrelate", |f| People::unrelate(
            &*f.r, f.ada, "u", "friend"
        )),
        write!("create", |f| People::create(&*f.r, Card::individual("New"))),
        write!("update", |f| People::update(
            &*f.r,
            f.bob,
            Card::individual("New")
        )),
        write!("delete", |f| People::delete(&*f.r, f.bob)),
        write!("import", |f| People::import(&*f.r, &JsContactFormat, "[]x")),
        write!("import a card", |f| People::import(
            &*f.r,
            &JsContactFormat,
            r#"{"@type":"Card","version":"1.0","uid":"i","name":{"full":"I"}}"#
        )),
        write!("create_group", |f| People::create_group(&*f.r, "New")),
        write!("add_member", |f| People::add_member(&*f.r, f.list, f.bob)),
        write!("remove_member", |f| People::remove_member(
            &*f.r, f.list, f.ada
        )),
        write!("confirm", |f| People::confirm_same_person(
            &*f.r, f.ada, f.bob
        )),
        write!("unlink", |f| People::unlink(&*f.r, f.link)),
        write!("define", |f| RoleBook::define_role(
            &*f.r,
            RoleDefinition::new("new", "New", "o", RoleScope::Personal)
        )),
        write!("undefine", |f| RoleBook::undefine_role(&*f.r, "coach")),
        write!("config", |f| RoleBook::load_role_config(&*f.r, "[]")),
        write!("assign", |f| RoleBook::assign_role(
            &*f.r,
            f.bob,
            "member",
            Some(CLUB),
            Map::new()
        )),
        write!("unassign", |f| RoleBook::unassign_role(&*f.r, f.held)),
        write!("mark_ai", |f| RoleBook::mark_ai(&*f.r, f.bob)),
    ]
}

async fn fixture() -> (Fixture, Arc<Gated>) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let gated = Arc::new(Gated::new(store));
    let r = PeopleRepo::with_backend(gated.clone(), gated.clone(), ME);
    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let bob = r
        .create(person("Bob", "b@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let carl = r
        .create(person("Carl", "c@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let list = r.create_group("List").await.unwrap().envelope.id;
    r.add_member(list, ada).await.unwrap();
    let held = r
        .assign_role(ada, "member", Some(CLUB), Map::new())
        .await
        .unwrap()
        .id;
    let link = r.confirm_same_person(ada, carl).await.unwrap().envelope.id;
    r.define_role(RoleDefinition::new(
        "coach",
        "Coach",
        "o",
        RoleScope::Personal,
    ))
    .await
    .unwrap();
    let r = Arc::new(r);
    (
        Fixture {
            r,
            ada,
            bob,
            list,
            held,
            link,
        },
        gated,
    )
}

#[tokio::test(flavor = "current_thread")]
async fn every_write_waits_for_the_one_before() {
    for (name, write) in writes() {
        let (f, gated) = fixture().await;
        gated.armed.store(1, Ordering::SeqCst);
        let r = f.r.clone();
        let ada = f.ada;
        let first = tokio::spawn(async move { People::relate(&*r, ada, "first", "friend").await });
        gated.arrived.notified().await;
        let before = gated.calls.load(Ordering::SeqCst);
        let second = tokio::spawn(write(f));
        for _ in 0..100 {
            tokio::task::yield_now().await;
        }
        assert_eq!(
            gated.calls.load(Ordering::SeqCst),
            before,
            "{name} did not wait its turn"
        );
        gated.gate.add_permits(1000);
        first.await.unwrap().unwrap();
        second.await.unwrap();
    }
}
