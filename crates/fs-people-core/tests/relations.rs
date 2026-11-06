//! What someone is to me lives on the card (RFC 9553 §2.1.8) and is not a role.

mod common;

use common::{person, repo};
use fs_people_core::Error;

#[tokio::test]
async fn relations_are_added_listed_and_removed() {
    let r = repo().await;
    let me = r
        .create(person("Me", "me@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.relate(me, "urn:uuid:ben", "friend").await.unwrap();
    r.relate(me, "urn:uuid:ben", "colleague").await.unwrap();
    let mut all = r.relations(me).await.unwrap();
    all.sort();
    assert_eq!(
        all,
        [
            ("urn:uuid:ben".to_string(), "colleague".to_string()),
            ("urn:uuid:ben".to_string(), "friend".to_string())
        ]
    );
    r.unrelate(me, "urn:uuid:ben", "friend").await.unwrap();
    assert_eq!(r.relations(me).await.unwrap().len(), 1);
    r.unrelate(me, "urn:uuid:ben", "colleague").await.unwrap();
    let card = r.get_card(me).await.unwrap().unwrap();
    assert!(card.related_to.is_empty());
}

#[tokio::test]
async fn relations_follow_the_standard_form_and_need_a_type() {
    let r = repo().await;
    let me = r
        .create(person("Me", "me@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.relate(me, "urn:uuid:ben", "friend").await.unwrap();
    let card = r.get_card(me).await.unwrap().unwrap();
    assert_eq!(
        card.to_value()["relatedTo"]["urn:uuid:ben"]["relation"]["friend"],
        true
    );
    assert!(matches!(
        r.relate(me, "urn:uuid:ben", " ").await,
        Err(Error::Conflict(_))
    ));
}
