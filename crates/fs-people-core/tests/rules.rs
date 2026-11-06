//! What a role asks of whoever holds it, at every door: `update`, `mark_ai`, a new configuration, `assign`.

mod common;

use common::{person, repo};
use fs_people_core::{Card, Error, PeopleRepo, RoleDefinition, RoleScope};
use fs_store::ObjectId;
use serde_json::{Map, json};

const CLUB: &str = "https://x/groups/club";

async fn ada(r: &PeopleRepo) -> ObjectId {
    r.create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id
}

fn conflict<T: std::fmt::Debug>(answer: fs_people_core::Result<T>) -> String {
    match answer {
        Err(Error::Conflict(said)) => said,
        other => panic!("expected a conflict: {other:?}"),
    }
}

#[tokio::test]
async fn a_marked_ai_keeps_its_kind_and_is_only_an_ai_with_both() {
    let r = repo().await;
    let id = ada(&r).await;
    r.mark_ai(id).await.unwrap();
    let mut card = r.get_card(id).await.unwrap().unwrap();

    card.kind = None;
    conflict(r.update(id, card.clone()).await);
    card.kind = Some("group".into());
    conflict(r.update(id, card.clone()).await);
    assert_eq!(r.get_card(id).await.unwrap().unwrap().kind(), "application");
    assert!(r.is_ai(id).await.unwrap());

    // An update that keeps the kind is no problem.
    card.kind = Some("application".into());
    card.name.as_mut().unwrap().full = Some("Ada 2".into());
    r.update(id, card).await.unwrap();

    // Plain software is `application` too, and is no AI.
    let tool = r
        .create(Card::application("Tool"))
        .await
        .unwrap()
        .envelope
        .id;
    assert!(!r.is_ai(tool).await.unwrap());
    assert!(!r.is_ai(ObjectId::new()).await.unwrap());
}

#[tokio::test]
async fn a_role_that_asks_another_kind_stops_the_marking_and_the_update() {
    let r = repo().await;
    let mut needs = RoleDefinition::new("guide", "Guide", "fs-people", RoleScope::Personal);
    needs.card_kind = Some("individual".into());
    r.define_role(needs).await.unwrap();
    let id = ada(&r).await;
    r.assign_role(id, "guide", None, Map::new()).await.unwrap();

    conflict(r.mark_ai(id).await);
    assert_eq!(r.get_card(id).await.unwrap().unwrap().kind(), "individual");
    assert!(!r.is_ai(id).await.unwrap());

    let mut card = r.get_card(id).await.unwrap().unwrap();
    card.kind = Some("org".into());
    conflict(r.update(id, card).await);
}

fn role(id: &str, scope: &str, kind: Option<&str>) -> serde_json::Value {
    json!({"id": id, "names": {"en": id}, "owner": "o", "scope": scope, "cardKind": kind})
}

#[tokio::test]
async fn a_configuration_cannot_orphan_or_misfit_who_holds_a_role() {
    let r = repo().await;
    let id = ada(&r).await;
    r.assign_role(id, "member", Some(CLUB), Map::new())
        .await
        .unwrap();

    let without = json!([role("other", "group", None)]);
    let said = conflict(r.store_role_config(&without.to_string()).await);
    assert!(said.contains("member"), "{said}");
    assert!(r.role("member").await.unwrap().is_some(), "nothing changed");

    let other_kind = json!([role("member", "group", Some("org"))]);
    conflict(r.store_role_config(&other_kind.to_string()).await);
    assert_eq!(r.role("member").await.unwrap().unwrap().card_kind, None);

    let personal_now = json!([role("member", "personal", None)]);
    conflict(r.store_role_config(&personal_now.to_string()).await);

    // Own roles count as there, and a configuration that fits goes through.
    let fits = json!([role("member", "group", Some("individual"))]);
    r.store_role_config(&fits.to_string()).await.unwrap();
    let kept = r.role("member").await.unwrap().unwrap();
    assert_eq!(kept.card_kind.as_deref(), Some("individual"));

    r.define_role(RoleDefinition::new("coach", "C", "o", RoleScope::Personal))
        .await
        .unwrap();
    r.assign_role(id, "coach", None, Map::new()).await.unwrap();
    r.store_role_config(&fits.to_string()).await.unwrap();
    assert_eq!(r.roles_of(id).await.unwrap().len(), 2);
}
