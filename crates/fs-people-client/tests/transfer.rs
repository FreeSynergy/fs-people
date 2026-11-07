//! vCard in and out through the trait `People`, including the release of sensitive cards (ADR 0029).
//!
//! Pattern: Integration tests against the trait; no stand-ins.

mod common;

use common::{Served, add, person};
use fs_people_core::{Error, People, VCardFormat};

const TWO: &str = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:urn:uuid:01a1aaef-ba80-7f7a-8dff-9f85769beae1\r\nFN:Ada\r\nEMAIL:ada@x.org\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nFN:Bob\r\nTEL:+49 30 1\r\nEND:VCARD\r\n";

#[tokio::test(flavor = "multi_thread")]
async fn a_vcard_is_imported_once_and_known_uids_are_skipped() {
    let served = Served::start().await;
    let people = served.client();
    let first = people.import_vcard(TWO).await.unwrap();
    assert_eq!((first.created.len(), first.skipped.len()), (2, 0));
    let again = people.import_vcard(TWO).await.unwrap();
    assert_eq!((again.created.len(), again.skipped.len()), (0, 2));
    let id = first.created[0];
    assert!(people.get(id).await.unwrap().is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_vcard_and_a_broken_one_are_reported_or_stored_not_lost() {
    let served = Served::start().await;
    let people = served.client();
    // The server gives a card without a UID one of its own, so the empty one is stored.
    let empty = format!("{TWO}BEGIN:VCARD\r\nVERSION:4.0\r\nEND:VCARD\r\n");
    let report = people.import_vcard(&empty).await.unwrap();
    assert_eq!(report.created.len() + report.failed.len(), 3, "{report:?}");
    // A card that never ends is not a card: it is named in `failed`.
    let broken = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Broken\r\n";
    let report = people.import_vcard(broken).await.unwrap();
    assert!(
        report.created.is_empty() && !report.failed.is_empty(),
        "{report:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn what_cannot_be_stored_is_in_the_report_and_the_others_arrive() {
    let dir = tempfile::tempdir().unwrap();
    let served = Served::start_with_file(&dir.path().join("p.db")).await;
    let people = served.client();
    let mut secret = person("Secret", "s@x.org");
    secret.mark_sensitive(["pd:SexualPreference"]);
    let text = format!(
        "{TWO}{}",
        fs_people_core::vcard::card_to_vcard(&secret).unwrap()
    );
    let report = people.import_vcard(&text).await.unwrap();
    assert_eq!(report.created.len(), 2, "{report:?}");
    assert_eq!(report.failed.len(), 1, "{report:?}");
    assert!(report.failed[0].contains("not encrypted"), "{report:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_card_is_exported_and_a_sensitive_one_only_on_release() {
    let served = Served::start().await;
    let people = served.client();
    let open = add(&people, person("Ada", "ada@x.org")).await;
    let exported = people
        .export_released(&VCardFormat, open, &[])
        .await
        .unwrap();
    assert!(exported.text.contains("FN:Ada"), "{}", exported.text);
    assert!(exported.withheld.is_empty());

    let mut secret = person("Secret", "s@x.org");
    secret.mark_sensitive(["pd:SexualPreference"]);
    let stored = people.create(secret).await.unwrap();
    let id = stored.envelope.id;
    let refused = people
        .export_released(&VCardFormat, id, &[])
        .await
        .unwrap_err();
    assert!(matches!(refused, Error::NotReleased(_)), "{refused:?}");

    let uid = stored.content["uid"].as_str().unwrap().to_string();
    let released = people
        .export_released(&VCardFormat, id, &[uid])
        .await
        .unwrap();
    assert!(released.text.contains("FN:Secret"));
}

#[tokio::test(flavor = "multi_thread")]
async fn exporting_a_missing_person_is_not_found() {
    let served = Served::start().await;
    let people = served.client();
    let nobody = fs_store::ObjectId::new();
    let got = people.export_released(&VCardFormat, nobody, &[]).await;
    assert!(
        matches!(got, Err(Error::NotFound(id)) if id == nobody),
        "{got:?}"
    );
}
