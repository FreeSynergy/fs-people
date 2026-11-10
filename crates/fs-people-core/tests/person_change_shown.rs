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

#[tokio::test]
async fn a_whole_form_sent_back_with_one_real_change_changes_only_that() {
    let r = repo().await;
    let id = stored(&r, Card::individual("Ada")).await;
    let card = r.get_card(id).await.unwrap().unwrap();
    let mut edit = Edit::new();
    for f in <Card as fs_types::DescribesType>::describe().fields {
        let shown = fs_people_core::person::values_of(&card, &f.name).unwrap();
        let values: Vec<&str> = shown.iter().map(String::as_str).collect();
        edit = edit.set(&[f.name.as_str()], f.basic, &values);
    }
    // The form's one real change: a phone number.
    for f in &mut edit.fields {
        if f.path[0] == "phones" {
            f.values = vec!["+4930123456".into()];
        }
    }
    let done = People::change(&r, id, &edit).await.unwrap();
    let after: Card = serde_json::from_value(done.content).unwrap();
    assert_eq!(after.phones.values().next().unwrap().number, "+4930123456");
    assert_eq!(after.name.unwrap().full.as_deref(), Some("Ada"));
}

#[tokio::test]
async fn an_empty_text_box_sent_back_for_a_read_only_field_is_no_change() {
    let r = repo().await;
    let id = stored(&r, Card::individual("Ada")).await;
    let before = r.get(id).await.unwrap().unwrap().content;
    let edit = Edit::new().set(&["prodId"], BasicType::Text, &[""]);
    let done = People::change(&r, id, &edit).await.unwrap();
    assert_eq!(done.content, before);
}

#[tokio::test]
async fn a_stored_blank_shown_and_sent_back_is_no_change() {
    let r = repo().await;
    let mut c = Card::individual("Ada");
    c.prod_id = Some(String::new());
    let id = stored(&r, c).await;
    let before = r.get(id).await.unwrap().unwrap().content;
    let edit = Edit::new().set(&["prodId"], BasicType::Text, &[""]);
    assert_eq!(People::change(&r, id, &edit).await.unwrap().content, before);
}

#[tokio::test]
async fn a_stored_blank_can_still_be_cleared() {
    let r = repo().await;
    let mut c = Card::individual("Ada");
    c.notes.insert(
        "n1".into(),
        serde_json::from_value(json!({"note": "   "})).unwrap(),
    );
    let id = stored(&r, c).await;
    let edit = Edit::new().clear(&["notes"], BasicType::MultilineText);
    let card: Card =
        serde_json::from_value(People::change(&r, id, &edit).await.unwrap().content).unwrap();
    assert!(card.notes.is_empty());
    // And a blank among values: sending only the real one drops the blank.
    let mut c = Card::individual("Bob");
    c.notes.insert(
        "n1".into(),
        serde_json::from_value(json!({"note": "a"})).unwrap(),
    );
    c.notes.insert(
        "n2".into(),
        serde_json::from_value(json!({"note": " "})).unwrap(),
    );
    let id = stored(&r, c).await;
    let edit = Edit::new().set(&["notes"], BasicType::MultilineText, &["a"]);
    let card: Card =
        serde_json::from_value(People::change(&r, id, &edit).await.unwrap().content).unwrap();
    assert_eq!(card.notes.len(), 1);
}
