//! `change`: a field sent back as the card shows it is no change (second review of the command).

mod common;

use common::repo;
use fs_people_core::{Card, Error, People};
use fs_types::{BasicType, Edit};
use serde_json::json;

async fn stored(r: &fs_people_core::PeopleRepo, card: Card) -> fs_objects::ObjectId {
    r.create(card).await.unwrap().envelope.id
}

fn code(e: &Error) -> Vec<(String, String)> {
    match e {
        Error::Invalid(issues) => issues
            .iter()
            .map(|i| (i.path.clone(), i.message.clone()))
            .collect(),
        other => panic!("{other}"),
    }
}

#[tokio::test]
async fn a_shown_value_that_is_older_than_the_rules_sent_back_is_not_a_change() {
    let r = repo().await;
    let card: Card = serde_json::from_value(json!({
        "@type": "Card", "version": "1.0", "uid": "urn:uuid:9f8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
        "kind": "example.com:robot",
        "keywords": {"Rust": true},
        "name": {"isOrdered": true, "sortAs": {"surname": "L"}, "components": [
            {"kind": "surname", "value": "Lovelace"}, {"kind": "separator", "value": ", "}, {"kind": "given", "value": "Ada"}]}
    }))
    .unwrap();
    let id = stored(&r, card).await;
    let before = r.get(id).await.unwrap().unwrap().content;
    let shown_name =
        fs_people_core::person::values_of(&r.get_card(id).await.unwrap().unwrap(), "name").unwrap();
    let edit = Edit::new()
        .set(&["kind"], BasicType::Choice, &["example.com:robot"])
        .set(&["keywords"], BasicType::Keywords, &["Rust"])
        .set(&["name"], BasicType::Name, &[shown_name[0].as_str()]);
    let done = People::change(&r, id, &edit).await.unwrap();
    assert_eq!(done.content, before);
}

#[tokio::test]
async fn a_path_named_twice_is_refused_even_when_one_of_them_is_the_shown_value() {
    let r = repo().await;
    let id = stored(&r, Card::individual("Ada")).await;
    let before = r.get(id).await.unwrap().unwrap().content;
    let edit = Edit::new()
        .set(&["kind"], BasicType::Choice, &["group"])
        .set(&["kind"], BasicType::Choice, &["individual"]);
    let err = People::change(&r, id, &edit).await.unwrap_err();
    assert_eq!(code(&err)[0].1, "duplicate-path");
    assert_eq!(r.get(id).await.unwrap().unwrap().content, before);
}
