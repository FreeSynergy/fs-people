//! Input of any shape and size: no panic, no card that cannot come back, nothing lost unreported.

mod common;

use common::{person, repo};
use fs_people_core::Error;
use serde_json::json;

#[tokio::test]
async fn a_relation_of_any_shape_can_be_edited() {
    let r = repo().await;
    let mut card = person("Ada", "a@x.org");
    card.related_to.insert("odd".into(), json!("not an object"));
    card.related_to
        .insert("odd2".into(), json!({"relation": 5}));
    let id = r.create(card).await.unwrap().envelope.id;
    r.relate(id, "odd", "friend").await.unwrap();
    r.relate(id, "odd2", "friend").await.unwrap();
    let found = r.relations(id).await.unwrap();
    let expected = [("odd", "friend"), ("odd2", "friend")].map(|(a, b)| (a.into(), b.into()));
    assert_eq!(found, expected);
}

#[tokio::test]
async fn a_card_cannot_grow_past_what_fits_an_answer() {
    let r = repo().await;
    let id = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let huge = "x".repeat(fs_people_core::MAX_CARD_BYTES);
    assert!(matches!(
        r.relate(id, &huge, "friend").await,
        Err(Error::TooLarge(_))
    ));
    assert!(r.relations(id).await.unwrap().is_empty());
}

#[tokio::test]
async fn lines_outside_a_complete_vcard_are_reported() {
    let r = repo().await;
    let good = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:u1\r\nFN:Ada\r\nEND:VCARD\r\n";
    let report = r
        .import_vcard(&format!("junk\r\n{good}BEGIN:VCARD\r\nFN:Cut off\r\n"))
        .await
        .unwrap();
    assert_eq!(report.created.len(), 1);
    assert_eq!(report.failed.len(), 1);
    assert!(report.failed[0].contains("3 lines"), "{report:?}");
    let nothing = r.import_vcard("no vcard at all").await.unwrap();
    assert!(
        nothing.created.is_empty() && nothing.failed.len() == 1,
        "{nothing:?}"
    );
    assert!(r.import_vcard(" \r\n").await.unwrap().failed.is_empty());
}
