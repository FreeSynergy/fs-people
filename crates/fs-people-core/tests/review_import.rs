//! Items 6 and 7 of the review: what an import reports when a file is only partly usable.
//!
//! Regression tests over a real in-memory `TursoStore`.

mod common;

use std::sync::Arc;

use common::{ME, person, repo};
use fs_people_core::PeopleRepo;
use fs_store::{ObjectId, Store, TursoStore};

const V7: &str = "01a1aaef-ba80-7f7a-8dff-0000000000ab";

fn vcard(uid: &str, name: &str) -> String {
    format!("BEGIN:VCARD\r\nVERSION:4.0\r\nUID:{uid}\r\nFN:{name}\r\nEND:VCARD\r\n")
}

#[tokio::test]
async fn item_6_a_block_without_an_end_is_reported_and_the_others_arrive() {
    let r = repo().await;
    let text = format!(
        "{}BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Cut Off\r\n",
        vcard(&format!("urn:uuid:{V7}"), "Whole")
    );
    let report = r.import_vcard(&text).await.unwrap();
    assert_eq!(report.created.len(), 1, "{report:?}");
    assert_eq!(report.failed.len(), 1, "{report:?}");
    let opened_twice = format!(
        "BEGIN:VCARD\r\nFN:Cut Off\r\n{}",
        vcard("urn:uuid:01a1aaef-ba80-7f7a-8dff-0000000000ac", "Second")
    );
    let report = r.import_vcard(&opened_twice).await.unwrap();
    assert_eq!(
        (report.created.len(), report.failed.len()),
        (1, 1),
        "{report:?}"
    );
}

#[tokio::test]
async fn item_7_a_store_refusal_is_one_failure_and_the_rest_of_the_file_arrives() {
    let dir = std::env::temp_dir().join(format!("fs-review-{}", ObjectId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    let store: Arc<dyn Store> = Arc::new(TursoStore::open(dir.join("c.db")).await.unwrap());
    let r = PeopleRepo::new(store, ME);
    let mut secret = person("Secret", "s@x.org");
    secret.mark_sensitive(["pd:HealthRecord"]);
    let cards = vec![
        person("Before", "b@x.org"),
        secret,
        person("After", "a@x.org"),
    ];
    let report = r
        .import_jscontact(&serde_json::to_string(&cards).unwrap())
        .await
        .unwrap();
    assert_eq!(report.created.len(), 2, "{report:?}");
    assert_eq!(report.failed.len(), 1, "{report:?}");
    assert!(report.failed[0].contains("not encrypted"), "{report:?}");
    std::fs::remove_dir_all(dir).ok();
}
