//! Numbers stay what they were: `details` and unknown properties are lossless, also through the store.

mod common;

use common::{person, repo};
use fs_people_core::{RoleDefinition, RoleScope};
use serde_json::{Map, Value, json};

const BIG: &str = "123456789012345678901234567890";

fn numbers() -> Value {
    serde_json::from_str(&format!(r#"{{"n":{BIG},"d":1.10,"e":1e400,"small":7}}"#)).unwrap()
}

fn text_of(v: &Value) -> String {
    v.to_string()
}

#[tokio::test]
async fn unknown_card_properties_keep_their_numbers() {
    let r = repo().await;
    let mut card = person("Ada", "a@x.org");
    card.extra.insert("example.org:n".into(), numbers());
    card.emails
        .get_mut("e1")
        .unwrap()
        .extra
        .insert("example.org:n".into(), numbers());
    let id = r.create(card).await.unwrap().envelope.id;

    let stored = r.get(id).await.unwrap().unwrap().content;
    let again = r.get_card(id).await.unwrap().unwrap();
    for text in [
        text_of(&stored["example.org:n"]),
        text_of(&stored["emails"]["e1"]["example.org:n"]),
        text_of(again.vendor("example.org:n").unwrap()),
    ] {
        assert!(
            text.contains(BIG) && text.contains("1.10") && text.contains("1e+400"),
            "{text}"
        );
    }
}

#[tokio::test]
async fn typed_numbers_inside_a_card_still_read() {
    let r = repo().await;
    let text = format!(
        r#"{{"@type":"Card","version":"1.0","uid":"u","name":{{"full":"A"}},"emails":{{"e":{{"address":"a@x.org","pref":3,"example.org:n":{BIG}}}}}}}"#
    );
    let report = r.import_jscontact(&text).await.unwrap();
    assert_eq!(report.failed, Vec::<String>::new());
    let card = r.get_card(report.created[0]).await.unwrap().unwrap();
    assert_eq!(card.emails["e"].pref, Some(3));
}

#[tokio::test]
async fn details_and_definitions_keep_their_numbers() {
    let r = repo().await;
    let id = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let mut own = RoleDefinition::new("o", "O", "fs-people", RoleScope::Public);
    own.extra.insert("example.org:n".into(), numbers());
    r.define_role(own).await.unwrap();
    assert!(
        text_of(
            r.role("o")
                .await
                .unwrap()
                .unwrap()
                .extra
                .get("example.org:n")
                .unwrap()
        )
        .contains(BIG)
    );

    let details: Map<String, Value> = numbers().as_object().unwrap().clone();
    let held = r.assign_role(id, "o", None, details.clone()).await.unwrap();
    assert_eq!(held.details, details);
    let read = r.roles_of(id).await.unwrap().remove(0).details;
    assert!(text_of(&json!(read)).contains(BIG), "{read:?}");
    assert_eq!(read, details);
}
