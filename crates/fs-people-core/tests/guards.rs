//! What a request from outside must never reach, and filters that combine.

mod common;

use common::{person, repo, setup};
use fs_people_core::{Card, Error, ListFilter, RoleDefinition, RoleRegistry, RoleScope};
use fs_store::{Envelope, ObjectId};
use serde_json::{Map, json};

const CLUB: &str = "https://x/groups/club";

fn not_found<T: std::fmt::Debug>(r: Result<T, Error>) -> bool {
    matches!(r, Err(Error::Store(fs_store::Error::NotFound(_))))
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
async fn people_with_a_role_appear_once_and_filters_combine() {
    let r = repo().await;
    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.create(person("Adam", "b@x.org")).await.unwrap();
    let org = r
        .create(Card::new("org", "Adacorp"))
        .await
        .unwrap()
        .envelope
        .id;
    for holder in [ada, org] {
        r.assign_role(holder, "member", Some(CLUB), Map::new())
            .await
            .unwrap();
    }
    r.assign_role(ada, "member", Some("https://x/groups/b"), Map::new())
        .await
        .unwrap();
    assert_eq!(r.people_with_role("member", None).await.unwrap().len(), 2);

    let names = |filter: ListFilter| {
        let r = r.clone();
        async move {
            let found = r.list(&filter).await.unwrap();
            found
                .into_iter()
                .map(|o| o.envelope.name)
                .collect::<Vec<_>>()
        }
    };
    let term = Some("ada".to_string());
    let all = names(ListFilter {
        term: term.clone(),
        ..Default::default()
    })
    .await;
    assert_eq!(all, ["Ada", "Adacorp", "Adam"]);
    let role = Some("member".to_string());
    let held = names(ListFilter {
        term: term.clone(),
        role: role.clone(),
        ..Default::default()
    })
    .await;
    assert_eq!(held, ["Ada", "Adacorp"]);
    let filter = ListFilter {
        term,
        role: role.clone(),
        kind: Some("org".into()),
        ..Default::default()
    };
    assert_eq!(names(filter).await, ["Adacorp"]);
    let filter = ListFilter {
        role,
        limit: Some(1),
        offset: 1,
        ..Default::default()
    };
    assert_eq!(names(filter).await, ["Adacorp"]);
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
    assert_eq!(ids, ["coach"], "one id, one definition");

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
    again.restore_role_config().await.unwrap();
    assert!(again.role("ai").await.unwrap().is_none());
    assert_eq!(again.role("coach").await.unwrap().unwrap().owner, "o");
}
