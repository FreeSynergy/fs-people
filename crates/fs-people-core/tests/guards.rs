//! What a request from outside must never reach, and filters that combine.

mod common;

use common::{person, repo, setup};
use fs_people_core::{Card, Error, RoleDefinition, RoleRegistry, RoleScope, full_id};
use fs_objects::{Envelope, ObjectId};
use serde_json::{Map, json};

const CLUB: &str = "https://x/groups/club";

fn not_found<T: std::fmt::Debug>(r: Result<T, Error>) -> bool {
    matches!(r, Err(Error::Store(fs_objects::Error::NotFound(_))))
}

#[tokio::test]
async fn removing_a_link_or_an_assignment_touches_nothing_else() {
    let r = repo().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let b = r
        .create(person("Ada K", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let held = r
        .assign_role(a, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    let link = r.confirm_same_person(a, b).await.unwrap().envelope.id;

    for victim in [a, held.id] {
        assert!(not_found(r.unlink(victim).await), "unlink {victim}");
    }
    for victim in [a, link] {
        assert!(
            not_found(r.unassign_role(victim).await),
            "unassign {victim}"
        );
    }
    assert!(not_found(r.unlink(ObjectId::new()).await));
    assert!(r.get(a).await.unwrap().is_some());
    assert_eq!(r.roles_of(a).await.unwrap().len(), 1);
    assert_eq!(r.linked_to(a).await.unwrap(), [b]);
}

#[tokio::test]
async fn a_uid_never_overwrites_the_object_with_that_id() {
    let (r, store) = setup().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let held = r
        .assign_role(a, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    let mut thief = Card::individual("Thief");
    thief.uid = held.id.to_string();
    assert!(matches!(r.create(thief).await, Err(Error::Conflict(_))));
    assert_eq!(r.roles_of(a).await.unwrap().len(), 1);

    let mut other = Card::individual("Other");
    other.uid = "not-a-uuid".into();
    let other = r.create(other).await.unwrap().envelope.id;
    let mut thief = Card::individual("Thief");
    thief.uid = other.to_string();
    assert!(matches!(r.create(thief).await, Err(Error::Conflict(_))));
    assert_eq!(r.get(other).await.unwrap().unwrap().envelope.name, "Other");

    // Gone is gone: the uid of a deleted card is free again.
    let mut back = Card::individual("Back");
    back.uid = Envelope::new("Profile", "x").id.to_string();
    let id = r.create(back.clone()).await.unwrap().envelope.id;
    r.delete(id).await.unwrap();
    assert!(store.get(id).await.unwrap().is_none());
    r.create(back).await.unwrap();
}

#[tokio::test]
async fn a_scope_is_an_iri_and_details_belong_to_own_roles() {
    let r = repo().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    for bad in ["personal", "", "club"] {
        let held = r.assign_role(a, "member", Some(bad), Map::new()).await;
        assert!(matches!(held, Err(Error::Invalid(_))), "{bad:?}");
    }
    let mut details = Map::new();
    details.insert("x".into(), json!(1));
    let held = r
        .assign_role(a, "member", Some(CLUB), details.clone())
        .await;
    assert!(matches!(held, Err(Error::Invalid(_))));
    assert!(r.roles_of(a).await.unwrap().is_empty());

    r.define_role(RoleDefinition::new(
        "coach",
        "Coach",
        "fs-people",
        RoleScope::Personal,
    ))
    .await
    .unwrap();
    let held = r
        .assign_role(a, "coach", None, details.clone())
        .await
        .unwrap();
    assert_eq!(held.details, details);
}

#[tokio::test]
async fn a_failed_ai_mark_leaves_the_card_alone() {
    let r = repo().await.with_roles(RoleRegistry::empty());
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    assert!(matches!(r.mark_ai(a).await, Err(Error::Conflict(_))));
    assert_eq!(r.get_card(a).await.unwrap().unwrap().kind(), "individual");
    assert!(!r.is_ai(a).await.unwrap());
}

#[tokio::test]
async fn a_stored_configuration_survives_and_wins_over_an_own_role() {
    let (r, store) = setup().await;
    r.define_role(RoleDefinition::new(
        "coach",
        "Own",
        "fs-people",
        RoleScope::Personal,
    ))
    .await
    .unwrap();
    let config =
        json!([{"id": "coach", "names": {"en": "Configured"}, "owner": "o", "scope": "group"}]);
    r.store_role_config(&config.to_string()).await.unwrap();
    let ids: Vec<_> = r.roles().await.unwrap().into_iter().map(|d| d.id).collect();
    let coaches = ids.iter().filter(|i| **i == full_id("coach")).count();
    assert_eq!(coaches, 1, "one id, one definition");
    assert!(
        ids.contains(&full_id("ai")),
        "the array merges, the shipped roles stay"
    );

    let bad = json!([{"id": "x", "names": {}, "owner": "o"}]);
    assert!(matches!(
        r.store_role_config(&bad.to_string()).await,
        Err(Error::Invalid(_))
    ));

    let again = fs_people_core::PeopleRepo::new(store, common::ME);
    assert!(
        again.role("ai").await.unwrap().is_some(),
        "not restored yet"
    );
    assert!(
        again
            .restore_role_config()
            .await
            .unwrap()
            .skipped()
            .is_empty()
    );
    assert!(
        again.role("ai").await.unwrap().is_some(),
        "the array merges over the shipped roles"
    );
    assert_eq!(again.role("coach").await.unwrap().unwrap().owner, "o");
}

#[tokio::test]
async fn a_refused_card_does_not_stop_the_import() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("objects.db");
    let store: std::sync::Arc<dyn fs_objects::Store> =
        std::sync::Arc::new(fs_objects::TursoStore::open(path).await.unwrap());
    let r = fs_people_core::PeopleRepo::new(store, common::ME);
    let cards = json!([
        {"@type": "Card", "version": "1.0", "uid": "secret", "name": {"full": "Secret"},
         "freesynergy.net:sensitive": ["pd:HealthRecord"]},
        {"@type": "Card", "version": "1.0", "uid": "plain", "name": {"full": "Plain"}},
    ]);
    let report = r.import_jscontact(&cards.to_string()).await.unwrap();
    assert_eq!(report.created.len(), 1);
    assert_eq!(report.failed.len(), 1);
    assert!(report.failed[0].starts_with("secret:"), "{report:?}");
}
