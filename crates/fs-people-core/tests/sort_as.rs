//! `name.sortAs`: the RFC form (map by component kind) is the only one written; the older string is
//! read, and rewritten at the next save.

mod common;

use common::repo;
use fs_objects::{Envelope, StoredObject};
use fs_people_core::{Card, People};
use serde_json::json;

fn with_sort_as(sort_as: serde_json::Value) -> Card {
    serde_json::from_value(json!({
        "@type": "Card", "version": "1.0", "uid": "urn:uuid:9f8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
        "name": {"full": "Ada", "sortAs": sort_as}
    }))
    .unwrap()
}

#[tokio::test]
async fn a_new_card_with_the_old_string_is_stored_as_a_map() {
    let r = repo().await;
    let stored = r.create(with_sort_as(json!("ADA"))).await.unwrap();
    assert_eq!(stored.content["name"]["sortAs"], json!({"surname": "ADA"}));
}

#[tokio::test]
async fn a_stored_old_card_is_read_as_it_is_and_rewritten_on_the_next_save() {
    // Put the older form into the store directly, as a version before this one wrote it.
    let (r2, store) = common::setup().await;
    let old = with_sort_as(json!("ADA"));
    let envelope = Envelope::new("Profile", "Ada");
    let id = envelope.id;
    let mut envelope = envelope;
    envelope.context = Some("people".into());
    let object = StoredObject::new(envelope, "application/jscontact+json", old.to_value())
        .with_index("uid", old.uid.as_str())
        .with_index("kind", "individual")
        .with_index("name_sort", "ada");
    store.put(object, common::ME).await.unwrap();
    let read = r2.get(id).await.unwrap().unwrap();
    assert_eq!(
        read.content["name"]["sortAs"],
        json!("ADA"),
        "reading changes nothing"
    );
    let card = r2.get_card(id).await.unwrap().unwrap();
    let saved = People::update(&r2, id, card).await.unwrap();
    assert_eq!(saved.content["name"]["sortAs"], json!({"surname": "ADA"}));
}

#[tokio::test]
async fn a_map_is_kept_as_it_is() {
    let r = repo().await;
    let map = json!({"surname": "Lovelace", "given": "Ada"});
    let stored = r.create(with_sort_as(map.clone())).await.unwrap();
    assert_eq!(stored.content["name"]["sortAs"], map);
}
