//! An array saved as configuration is resolved to the file of our base: running and after a restart
//! the same roles are in force. Restoring says what it skipped.

mod common;

use common::{ME, setup};
use fs_objects::{Envelope, StoredObject};
use fs_people_core::{CONTEXT, Error, PeopleRepo, RoleDirectory, RoleRegistry};
use serde_json::{Map, json};

const CLUB: &str = "https://local.invalid/groups/club";

fn own(id: &str) -> serde_json::Value {
    json!({"id": id, "names": {"en": id}, "owner": "kal", "scope": "group"})
}

fn ours(roles: &[serde_json::Value]) -> String {
    json!({
        "@context": [{"@base": "https://freesynergy.net/ns/roles/"}],
        "id": "", "type": "skos:ConceptScheme", "roles": roles
    })
    .to_string()
}

fn ids(all: Vec<fs_people_core::RoleDefinition>) -> Vec<String> {
    let mut ids: Vec<String> = all.into_iter().map(|d| d.id).collect();
    ids.sort();
    ids
}

#[tokio::test]
async fn a_held_role_survives_an_array_and_a_restart() {
    let (r, store) = setup().await;
    r.store_role_config(&ours(&[own("ai"), own("trainer")]))
        .await
        .unwrap();
    let ada = r
        .create(common::person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.assign_role(ada, "trainer", Some(CLUB), Map::new())
        .await
        .unwrap();
    r.store_role_config(&json!([own("coach")]).to_string())
        .await
        .unwrap();
    let running = ids(r.roles().await.unwrap());

    let again = PeopleRepo::new(store, ME);
    let restored = again.restore_role_config().await.unwrap();
    assert!(restored.skipped().is_empty());
    assert_eq!(ids(again.roles().await.unwrap()), running);
    assert!(again.role("trainer").await.unwrap().is_some());
}

#[tokio::test]
async fn the_second_scenario_is_the_same_running_and_restarted() {
    let (r, store) = setup().await;
    let ai = json!({"id": "ai", "names": {"en": "AI"}, "owner": "fs-people", "scope": "public"});
    r.store_role_config(&ours(&[ai, own("mine")]))
        .await
        .unwrap();
    assert!(r.role("member").await.unwrap().is_none());
    r.store_role_config(&json!([own("coach")]).to_string())
        .await
        .unwrap();
    let running = ids(r.roles().await.unwrap());
    assert_eq!(running.len(), 3);

    let again = PeopleRepo::new(store, ME);
    assert!(
        again
            .restore_role_config()
            .await
            .unwrap()
            .skipped()
            .is_empty()
    );
    assert_eq!(ids(again.roles().await.unwrap()), running);
    assert!(again.role("member").await.unwrap().is_none());
}

#[tokio::test]
async fn an_array_cannot_drop_what_is_held_and_cannot_orphan() {
    let (r, _) = setup().await;
    let ada = r
        .create(common::person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.assign_role(ada, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    // Replacing the file without `member` is refused; the array only adds, so it passes.
    let without = ours(&[own("other")]);
    assert!(matches!(
        r.store_role_config(&without).await,
        Err(Error::Conflict(_))
    ));
    r.store_role_config(&json!([own("other")]).to_string())
        .await
        .unwrap();
    assert!(r.role("member").await.unwrap().is_some());
}

#[tokio::test]
async fn an_owner_that_is_nothing_after_migration_becomes_unknown() {
    let dir = RoleRegistry::empty();
    let old = json!([{"id": "x", "names": {"en": "X"}, "owner": "?"}]).to_string();
    assert!(dir.load(&old).is_err(), "new input stays strict");
    dir.load_stored(&old).unwrap();
    assert_eq!(dir.find("x").unwrap().owner, "unknown");
}

#[tokio::test]
async fn restoring_reports_what_it_skipped() {
    let (_, store) = setup().await;
    let mut env = Envelope::new("RoleConfig", "broken");
    env.context = Some(CONTEXT.into());
    let broken = StoredObject::new(env, "application/json", json!({"nonsense": 1}));
    store.put(broken, ME).await.unwrap();
    let report = PeopleRepo::new(store, ME)
        .restore_role_config()
        .await
        .unwrap();
    assert_eq!(report.skipped().len(), 1);
}
