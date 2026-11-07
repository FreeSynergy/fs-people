//! A list holds `uid`s, and a relation is open: what stays, what goes.

mod common;

use common::{person, repo};
use fs_people_core::{Card, Error, PeopleRepo};
use fs_objects::ObjectId;
use serde_json::json;

async fn a_list_with_ada(r: &PeopleRepo) -> (ObjectId, ObjectId, String) {
    let card = r.create(person("Ada", "a@x.org")).await.unwrap();
    let id = card.envelope.id;
    let uid = r.get_card(id).await.unwrap().unwrap().uid;
    let list = r.create_group("L").await.unwrap().envelope.id;
    r.add_member(list, id).await.unwrap();
    (list, id, uid)
}

#[tokio::test]
async fn a_deleted_member_leaves_the_list_and_does_not_come_back() {
    let r = repo().await;
    let (list, id, uid) = a_list_with_ada(&r).await;
    r.delete(id).await.unwrap();
    assert!(r.get_card(list).await.unwrap().unwrap().members.is_empty());

    let mut again = person("Ada again", "a@x.org");
    again.uid = uid;
    r.create(again).await.unwrap();
    assert!(r.members(list).await.unwrap().is_empty(), "came back");
}

#[tokio::test]
async fn removal_goes_by_the_uid_in_any_spelling_and_also_when_the_card_is_gone() {
    let r = repo().await;
    let list = r.create_group("L").await.unwrap().envelope.id;
    let plain = "01a1aaef-ba80-7f7a-8dff-0000000000ab";
    let mut card = Card::individual("Ada");
    card.uid = plain.into();
    let id = r.create(card).await.unwrap().envelope.id;
    let mut group = r.get_card(list).await.unwrap().unwrap();
    group
        .members
        .insert(format!("urn:uuid:{}", plain.to_uppercase()), true);
    r.update(list, group.clone()).await.unwrap();
    r.remove_member(list, id).await.unwrap();
    assert!(r.get_card(list).await.unwrap().unwrap().members.is_empty());

    // A member of an imported list, whose card never was here.
    r.update(list, group).await.unwrap();
    let gone: ObjectId = plain.parse().unwrap();
    r.delete(id).await.unwrap();
    let mut group = r.get_card(list).await.unwrap().unwrap();
    group.members.insert(plain.into(), true);
    r.update(list, group).await.unwrap();
    r.remove_member(list, gone).await.unwrap();
    assert!(r.get_card(list).await.unwrap().unwrap().members.is_empty());
    let stranger = ObjectId::new();
    assert!(matches!(
        r.remove_member(list, stranger).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn a_card_that_is_no_list_says_so_everywhere() {
    let r = repo().await;
    let (_, id, _) = a_list_with_ada(&r).await;
    let other = r
        .create(person("Bob", "b@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    assert!(matches!(
        r.add_member(id, other).await,
        Err(Error::Conflict(_))
    ));
    assert!(matches!(
        r.remove_member(id, other).await,
        Err(Error::Conflict(_))
    ));
    assert!(matches!(r.members(id).await, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn unrelate_keeps_what_is_not_its_own_and_an_untyped_relation_shows() {
    let r = repo().await;
    let mut card = person("Ada", "a@x.org");
    card.related_to.insert(
        "with-more".into(),
        json!({"@type": "Relation", "relation": {"friend": true}, "example.org:note": "keep"}),
    );
    card.related_to.insert(
        "two".into(),
        json!({"relation": {"friend": true, "colleague": true}}),
    );
    card.related_to
        .insert("untyped".into(), json!({"@type": "Relation"}));
    let id = r.create(card).await.unwrap().envelope.id;

    let pairs = |list: &[(&str, &str)]| -> Vec<(String, String)> {
        list.iter()
            .map(|(a, b)| ((*a).into(), (*b).into()))
            .collect()
    };
    assert_eq!(
        r.relations(id).await.unwrap(),
        pairs(&[
            ("two", "colleague"),
            ("two", "friend"),
            ("untyped", ""),
            ("with-more", "friend")
        ])
    );

    r.unrelate(id, "with-more", "friend").await.unwrap();
    let card = r.get_card(id).await.unwrap().unwrap();
    assert_eq!(card.related_to["with-more"]["example.org:note"], "keep");
    r.unrelate(id, "two", "friend").await.unwrap();
    r.unrelate(id, "untyped", "friend").await.unwrap();
    assert_eq!(
        r.relations(id).await.unwrap(),
        pairs(&[("two", "colleague"), ("untyped", ""), ("with-more", "")])
    );
    r.unrelate(id, "untyped", "").await.unwrap();
    r.unrelate(id, "two", "").await.unwrap();
    assert_eq!(
        r.relations(id).await.unwrap(),
        pairs(&[("two", "colleague"), ("with-more", "")]),
        "an empty relation removes only an entry that names none"
    );
    r.unrelate(id, "two", "colleague").await.unwrap();
    let card = r.get_card(id).await.unwrap().unwrap();
    assert!(!card.related_to.contains_key("two"));
}

#[tokio::test]
async fn a_word_with_digits_is_no_phone_number() {
    let r = repo().await;
    let mut card = person("Ada", "a@x.org");
    card.add_phone("+49 30 555");
    r.create(card).await.unwrap();
    assert!(r.search("Room 555").await.unwrap().is_empty());
    for hit in ["555", "+49 30", "49-30 555"] {
        assert_eq!(r.search(hit).await.unwrap().len(), 1, "{hit}");
    }
    assert!(r.search("(030) 777").await.unwrap().is_empty());
}
