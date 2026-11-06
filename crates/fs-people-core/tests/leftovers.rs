//! What a delete that stopped halfway left must not pass to a new card of that id.

mod common;

use common::{ME, person, setup};
use fs_people_core::{Card, Error};
use serde_json::Map;

const CLUB: &str = "https://x/groups/club";
const V7: &str = "01a1aaef-ba80-7f7a-8dff-0000000000ab";

#[tokio::test]
async fn a_new_card_of_an_id_does_not_inherit_what_a_delete_left() {
    let (r, store) = setup().await;
    let mut card = person("Ada", "a@x.org");
    card.uid = V7.into();
    let id = r.create(card.clone()).await.unwrap().envelope.id;
    r.assign_role(id, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    // A delete that stopped after the card.
    store.delete(id, ME).await.unwrap();

    let said = r.create(card.clone()).await.unwrap_err();
    assert!(matches!(said, Error::Conflict(_)), "{said}");
    let report = r
        .import_jscontact(&serde_json::to_string(&card).unwrap())
        .await
        .unwrap();
    assert_eq!(report.created.len(), 0);
    assert_eq!(report.failed.len(), 1, "{report:?}");

    r.delete(id).await.unwrap();
    r.create(card).await.unwrap();
    assert!(r.roles_of(id).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_new_group_of_an_id_does_not_inherit_the_roles_held_within_it() {
    let (r, store) = setup().await;
    let mut group = Card::group("Club");
    group.uid = V7.into();
    let id = r.create(group.clone()).await.unwrap().envelope.id;
    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let iri = r.get(id).await.unwrap().unwrap().envelope.iri.unwrap();
    r.assign_role(ada, "member", Some(&iri), Map::new())
        .await
        .unwrap();
    store.delete(id, ME).await.unwrap();
    assert!(matches!(r.create(group).await, Err(Error::Conflict(_))));
}
