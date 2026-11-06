//! An AI is a marked participant, not a restricted one (ADR 0054).

mod common;

use common::{person, repo};
use fs_people_core::{AI_ROLE, Card, Error};
use serde_json::{Map, json};

#[tokio::test]
async fn marking_sets_kind_and_role_and_the_kind_travels_with_the_card() {
    let r = repo().await;
    let id = r
        .create(Card::application("Helper"))
        .await
        .unwrap()
        .envelope
        .id;
    assert!(!r.is_ai(id).await.unwrap(), "plain software is not an AI");
    r.mark_ai(id).await.unwrap();
    assert!(r.is_ai(id).await.unwrap());
    assert_eq!(r.get_card(id).await.unwrap().unwrap().kind(), "application");
    assert!(
        r.export_vcard(id)
            .await
            .unwrap()
            .to_lowercase()
            .contains("kind:application")
    );
    assert_eq!(r.people_with_role(AI_ROLE, None).await.unwrap(), [id]);
}

#[tokio::test]
async fn an_existing_card_can_be_marked_and_marking_twice_changes_nothing() {
    let r = repo().await;
    let id = r
        .create(person("Bot", "bot@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let one = r.mark_ai(id).await.unwrap();
    let two = r.mark_ai(id).await.unwrap();
    assert_eq!(one.id, two.id);
    assert_eq!(r.get_card(id).await.unwrap().unwrap().kind(), "application");
}

#[tokio::test]
async fn the_role_needs_the_kind_and_a_group_is_no_ai() {
    let r = repo().await;
    let human = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    assert!(matches!(
        r.assign_role(human, AI_ROLE, None, Map::new()).await,
        Err(Error::Conflict(_))
    ));
    let group = r.create_group("Team").await.unwrap().envelope.id;
    assert!(matches!(r.mark_ai(group).await, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn an_ai_does_everything_a_person_does() {
    let r = repo().await;
    let id = r
        .create(Card::application("Helper"))
        .await
        .unwrap()
        .envelope
        .id;
    r.mark_ai(id).await.unwrap();
    let group = r.create_group("Team").await.unwrap().envelope.id;
    r.add_member(group, id).await.unwrap();
    assert_eq!(r.members(group).await.unwrap().len(), 1);
    r.assign_role(id, "member", Some("https://x/groups/t"), Map::new())
        .await
        .unwrap();
    assert_eq!(r.roles_of(id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn role_details_stay_lossless_for_what_comes_later() {
    // The operator of an AI will live somewhere; its form is open (ADR 0054). Nothing here blocks it.
    let r = repo().await;
    let id = r
        .create(Card::application("Helper"))
        .await
        .unwrap()
        .envelope
        .id;
    let mut details = Map::new();
    details.insert("example.org:anything".into(), json!({"a": 1}));
    r.assign_role(id, AI_ROLE, None, details.clone())
        .await
        .unwrap();
    assert_eq!(r.roles_of(id).await.unwrap()[0].details, details);
}
