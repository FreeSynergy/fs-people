//! A second card with a uid that is taken answers with the id of the card that is there.

use fs_objects::TursoStore;
use fs_people_core::{Card, Error, Kind, PeopleRepo};
use std::sync::Arc;

#[tokio::test]
async fn a_duplicate_uid_names_the_existing_card() {
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let r = PeopleRepo::new(store, "https://example.org/users/admin");
    let first = r.create(Card::individual("First")).await.unwrap();
    let mut again = Card::individual("Second");
    again.uid = first.content["uid"].as_str().unwrap().to_string();
    let got = r.create(again).await;
    let Err(e) = got else {
        panic!("a duplicate was stored")
    };
    assert!(
        matches!(e, Error::Exists(id) if id == first.envelope.id),
        "{e:?}"
    );
    assert!(e.is_conflict());
    let subject = e.subject().unwrap();
    assert_eq!(
        (subject.kind, subject.id),
        (Kind::Person, first.envelope.id.to_string())
    );
}
