//! `get_by_uid` and cards stored before the index `uid_key` existed.
//!
//! Pattern: Regression tests over a real in-memory `TursoStore`; an older card is made by storing it
//! again without that index field.

mod common;

use std::sync::Arc;

use common::{ME, person};
use fs_objects::{Store, TursoStore};
use fs_people_core::{Card, PeopleRepo};

const V4: &str = "9f8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d";

async fn with_old_card(uid: &str) -> (PeopleRepo, Arc<dyn Store>) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let r = PeopleRepo::new(store.clone(), ME);
    let mut card = Card::individual("Old");
    card.uid = uid.into();
    let mut stored = r.create(card).await.unwrap();
    stored.index.remove("uid_key");
    store.put(stored, ME).await.unwrap();
    (r, store)
}

fn spellings() -> Vec<String> {
    let up = V4.to_uppercase();
    let plain = V4.replace('-', "");
    vec![
        V4.to_string(),
        format!("urn:uuid:{V4}"),
        up.clone(),
        format!("urn:uuid:{up}"),
        format!("URN:UUID:{up}"),
        plain.clone(),
        plain.to_uppercase(),
    ]
}

#[tokio::test]
async fn a_card_without_the_index_is_still_found_in_any_spelling() {
    for stored in spellings() {
        let (r, _store) = with_old_card(&stored).await;
        for asked in spellings() {
            let mut again = Card::individual("Second");
            again.uid.clone_from(&asked);
            let got = r.create(again).await;
            assert!(
                matches!(got, Err(ref e) if e.is_conflict()),
                "{stored} then {asked}: {got:?}"
            );
            let vcard =
                format!("BEGIN:VCARD\r\nVERSION:4.0\r\nUID:{asked}\r\nFN:Second\r\nEND:VCARD\r\n");
            let report = r.import_vcard(&vcard).await.unwrap();
            assert_eq!(report.skipped.len(), 1, "{stored} then {asked}: {report:?}");
        }
        assert_eq!(
            r.list(&fs_people_core::ListFilter::default())
                .await
                .unwrap()
                .len(),
            1,
            "{stored}"
        );
    }
}

#[tokio::test]
async fn another_uid_that_only_contains_the_uuid_is_not_the_same_card() {
    let (r, _store) = with_old_card(&format!("prefix-{V4}-suffix")).await;
    assert!(r.get_by_uid(V4).await.unwrap().is_none());
    let mut other = Card::individual("Other");
    other.uid = format!("urn:uuid:{V4}");
    r.create(other).await.unwrap();
    assert_eq!(
        r.list(&fs_people_core::ListFilter::default())
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn get_by_uid_goes_by_the_uid_not_by_the_object_id_and_create_still_will_not_overwrite() {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let r = PeopleRepo::new(store, ME);
    let mut card = person("Plain", "p@x.org");
    card.uid = "plain-uid".into();
    let stored = r.create(card).await.unwrap();
    let id = stored.envelope.id.to_string();

    assert!(
        r.get_by_uid(&id).await.unwrap().is_none(),
        "its uid is another"
    );
    assert!(r.get_by_uid("plain-uid").await.unwrap().is_some());

    let mut clash = Card::individual("Clash");
    clash.uid = format!("urn:uuid:{id}");
    let got = r.create(clash.clone()).await;
    assert!(matches!(got, Err(ref e) if e.is_conflict()), "{got:?}");
    let text = serde_json::to_string(&clash).unwrap();
    let report = r.import_jscontact(&text).await.unwrap();
    assert_eq!(
        report.failed.len(),
        1,
        "reported, not an overwrite: {report:?}"
    );
    let still = r.get(stored.envelope.id).await.unwrap().unwrap();
    assert_eq!(still.envelope.name, "Plain");
}
