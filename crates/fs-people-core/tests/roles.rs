//! One person, several roles (ADR 0052); definitions are data, ours and yours alike.

mod common;

use common::{person, repo};
use fs_people_core::{Error, RoleDefinition, RoleRegistry, RoleScope, full_id};
use serde_json::{Map, json};

const CLUB: &str = "https://local.invalid/groups/club";

#[tokio::test]
async fn one_person_has_many_roles_without_a_second_record() {
    let r = repo().await;
    let ada = r
        .create(person("Ada", "ada@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.assign_role(ada, "customer", Some(CLUB), Map::new())
        .await
        .unwrap();
    r.assign_role(ada, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    let mut held: Vec<_> = r
        .roles_of(ada)
        .await
        .unwrap()
        .into_iter()
        .map(|a| a.role)
        .collect();
    held.sort();
    assert_eq!(held, ["customer", "member"].map(full_id));
    assert_eq!(
        r.list(&fs_people_core::ListFilter::default())
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        r.people_with_role("member", Some(CLUB)).await.unwrap(),
        [ada]
    );
    assert_eq!(
        r.people_with_role("member", Some("https://x/groups/other"))
            .await
            .unwrap(),
        [] as [fs_objects::ObjectId; 0]
    );
}

#[tokio::test]
async fn assigning_twice_keeps_one_and_unknown_roles_are_refused() {
    let r = repo().await;
    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let one = r
        .assign_role(ada, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    let two = r
        .assign_role(ada, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    assert_eq!(one.id, two.id);
    assert!(matches!(
        r.assign_role(ada, "wizard", None, Map::new()).await,
        Err(Error::Conflict(_))
    ));
    r.unassign_role(one.id).await.unwrap();
    assert_eq!(
        r.roles_of(ada).await.unwrap(),
        [] as [fs_people_core::RoleAssignment; 0]
    );
}

#[tokio::test]
async fn a_role_never_leaves_with_the_card() {
    let r = repo().await;
    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.assign_role(ada, "customer", Some(CLUB), Map::new())
        .await
        .unwrap();
    let vcard = r.export_vcard(ada).await.unwrap();
    assert!(!vcard.to_lowercase().contains("customer"), "{vcard}");
    let card = serde_json::to_string(&r.get_card(ada).await.unwrap().unwrap()).unwrap();
    assert!(!card.contains("customer"), "{card}");
}

#[tokio::test]
async fn own_roles_are_defined_like_ours_and_survive_as_objects() {
    let r = repo().await;
    let coach = RoleDefinition::new("coach", "Coach", "my-club", RoleScope::Group);
    r.define_role(coach.clone()).await.unwrap();
    assert_eq!(r.role("coach").await.unwrap(), Some(coach.clone()));
    assert!(
        r.roles()
            .await
            .unwrap()
            .iter()
            .any(|d| d.id == full_id("coach"))
    );
    assert!(matches!(
        r.define_role(coach).await,
        Err(Error::Conflict(_))
    ));

    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let held = r
        .assign_role(ada, "coach", Some(CLUB), Map::new())
        .await
        .unwrap();
    assert!(matches!(
        r.undefine_role("coach").await,
        Err(Error::Conflict(_))
    ));
    r.unassign_role(held.id).await.unwrap();
    r.undefine_role("coach").await.unwrap();
    assert_eq!(r.role("coach").await.unwrap(), None);
}

#[tokio::test]
async fn role_config_is_checked_first_then_swapped() {
    let r = repo().await.with_roles(RoleRegistry::empty());
    assert_eq!(
        r.roles().await.unwrap(),
        [] as [fs_people_core::RoleDefinition; 0]
    );
    let bad = json!([{"id": "x", "names": {}, "owner": "o"}]).to_string();
    assert!(matches!(r.swap_role_config(&bad), Err(Error::Invalid(_))));
    assert!(r.roles().await.unwrap().is_empty(), "nothing changed");
    let good = json!([{"id": "x", "names": {"en": "X"}, "owner": "o", "scope": "public"}]);
    r.swap_role_config(&good.to_string()).unwrap();
    assert_eq!(r.role("x").await.unwrap().unwrap().scope, RoleScope::Public);
    // a configured role cannot be removed as if it were your own
    assert!(matches!(
        r.undefine_role("x").await,
        Err(Error::Conflict(_))
    ));
}

#[tokio::test]
async fn deleting_a_person_ends_their_roles() {
    let r = repo().await;
    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.assign_role(ada, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    r.delete(ada).await.unwrap();
    assert_eq!(
        r.people_with_role("member", None).await.unwrap(),
        [] as [fs_objects::ObjectId; 0]
    );
}

#[tokio::test]
async fn role_assignments_do_not_disturb_same_person_links() {
    let r = repo().await;
    let a = r.create(person("A", "a@x.org")).await.unwrap().envelope.id;
    r.assign_role(a, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    assert_eq!(
        r.linked_to(a).await.unwrap(),
        [] as [fs_objects::ObjectId; 0]
    );
}
