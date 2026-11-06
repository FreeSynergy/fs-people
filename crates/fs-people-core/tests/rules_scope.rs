//! How far a role reaches decides its scope; who holds it personally; details that are not dropped.

mod common;

use common::{person, repo};
use fs_people_core::{Error, PeopleRepo, RoleDefinition, RoleScope};
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

fn role(id: &str, scope: &str, kind: Option<&str>) -> serde_json::Value {
    json!({"id": id, "names": {"en": id}, "owner": "o", "scope": scope, "cardKind": kind})
}

fn scope_refused<T: std::fmt::Debug>(answer: fs_people_core::Result<T>) {
    match answer {
        Err(Error::Invalid(issues)) => assert_eq!(issues[0].path, "/scope"),
        other => panic!("expected /scope: {other:?}"),
    }
}

#[tokio::test]
async fn how_far_a_role_reaches_decides_its_scope() {
    let r = repo().await;
    let id = ada(&r).await;
    for (role_id, scope) in [
        ("g", RoleScope::Group),
        ("p", RoleScope::Personal),
        ("o", RoleScope::Public),
    ] {
        r.define_role(RoleDefinition::new(role_id, role_id, "fs-people", scope))
            .await
            .unwrap();
    }
    scope_refused(r.assign_role(id, "g", None, Map::new()).await);
    assert_eq!(
        r.assign_role(id, "g", Some(CLUB), Map::new())
            .await
            .unwrap()
            .scope
            .as_deref(),
        Some(CLUB)
    );

    scope_refused(r.assign_role(id, "p", Some(CLUB), Map::new()).await);
    assert_eq!(
        r.assign_role(id, "p", None, Map::new())
            .await
            .unwrap()
            .scope,
        None
    );

    assert_eq!(
        r.assign_role(id, "o", None, Map::new())
            .await
            .unwrap()
            .scope,
        None
    );
    assert_eq!(
        r.assign_role(id, "o", Some(CLUB), Map::new())
            .await
            .unwrap()
            .scope
            .as_deref(),
        Some(CLUB)
    );
}

#[tokio::test]
async fn mark_ai_stays_allowed_when_the_role_is_configured_for_groups() {
    let r = repo().await;
    let config = json!([role("ai", "group", Some("application"))]);
    r.store_role_config(&config.to_string()).await.unwrap();
    let id = ada(&r).await;
    let held = r.mark_ai(id).await.unwrap();
    assert_eq!(held.scope, None);
    assert!(r.is_ai(id).await.unwrap());
}

#[tokio::test]
async fn personal_and_group_holders_stay_apart_and_a_name_is_no_scope() {
    let r = repo().await;
    let (a, b) = (
        ada(&r).await,
        r.create(person("Bob", "b@x.org"))
            .await
            .unwrap()
            .envelope
            .id,
    );
    r.define_role(RoleDefinition::new(
        "o",
        "O",
        "fs-people",
        RoleScope::Public,
    ))
    .await
    .unwrap();
    r.assign_role(a, "o", None, Map::new()).await.unwrap();
    r.assign_role(b, "o", Some(CLUB), Map::new()).await.unwrap();
    assert_eq!(r.people_with_personal_role("o").await.unwrap(), [a]);
    assert_eq!(r.people_with_role("o", Some(CLUB)).await.unwrap(), [b]);
    assert_eq!(r.people_with_role("o", None).await.unwrap().len(), 2);
    for not_an_iri in ["personal", "club"] {
        scope_refused(r.people_with_role("o", Some(not_an_iri)).await);
    }
}

#[tokio::test]
async fn details_are_not_dropped_and_an_unknown_role_is_not_found() {
    let r = repo().await;
    let id = ada(&r).await;
    r.define_role(RoleDefinition::new(
        "o",
        "O",
        "fs-people",
        RoleScope::Public,
    ))
    .await
    .unwrap();
    let details = |v: i32| {
        let mut d = Map::new();
        d.insert("v".into(), json!(v));
        d
    };
    let first = r.assign_role(id, "o", None, details(1)).await.unwrap();
    assert_eq!(
        r.assign_role(id, "o", None, details(1)).await.unwrap().id,
        first.id
    );
    assert_eq!(
        r.assign_role(id, "o", None, Map::new())
            .await
            .unwrap()
            .details,
        details(1)
    );
    conflict(r.assign_role(id, "o", None, details(2)).await);
    assert_eq!(r.roles_of(id).await.unwrap()[0].details, details(1));

    assert!(matches!(
        r.undefine_role("nobody").await,
        Err(Error::UnknownRole(_))
    ));
}
