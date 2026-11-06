//! The findings of the independent review (2026-11-06), each as a test on the real core.
//!
//! Pattern: Regression tests over a real in-memory `TursoStore`; a failing store is a Decorator
//! around the real one.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use common::{ME, person, repo};
use fs_people_core::index::{normalise_email, normalise_phone};
use fs_people_core::{Card, Error, PeopleRepo};
use fs_store::traits::{ObjectQuery, ObjectRepository};
use fs_store::{Activity, LogEntry, ObjectId, Query, Store, StoredObject, TursoStore};
use serde_json::json;

const V7: &str = "01a1aaef-ba80-7f7a-8dff-0000000000ab";
const V4: &str = "9f8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d";

fn card_with_uid(uid: &str, name: &str) -> Card {
    let mut card = Card::individual(name);
    card.uid = uid.into();
    card
}

fn vcard(uid: &str, name: &str) -> String {
    format!("BEGIN:VCARD\r\nVERSION:4.0\r\nUID:{uid}\r\nFN:{name}\r\nEND:VCARD\r\n")
}

/// Every spelling of one identity, for a v7 and a v4 UUID.
fn spellings() -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for id in [V7, V4] {
        let up = id.to_uppercase();
        let forms = [
            format!("urn:uuid:{id}"),
            id.to_string(),
            format!("urn:uuid:{up}"),
            up.clone(),
            format!("URN:UUID:{up}"),
        ];
        for a in &forms {
            for b in &forms {
                if a != b {
                    pairs.push((a.clone(), b.clone()));
                }
            }
        }
    }
    pairs
}

#[tokio::test]
async fn item_1_create_refuses_another_spelling_of_a_known_identity() {
    for (first, second) in spellings() {
        let r = repo().await;
        r.create(card_with_uid(&first, "First")).await.unwrap();
        let again = r.create(card_with_uid(&second, "Second")).await;
        assert!(
            matches!(again, Err(Error::Conflict(_))),
            "{first} then {second}: {again:?}"
        );
        let all = r.list(&Default::default()).await.unwrap();
        assert_eq!(all.len(), 1, "{first} then {second}");
        assert_eq!(all[0].envelope.name, "First", "{first} then {second}");
    }
}

#[tokio::test]
async fn item_1_import_skips_another_spelling_of_a_known_identity() {
    for (first, second) in spellings() {
        let r = repo().await;
        r.create(card_with_uid(&first, "First")).await.unwrap();
        let report = r.import_vcard(&vcard(&second, "Second")).await.unwrap();
        assert_eq!(report.skipped.len(), 1, "{first} then {second}: {report:?}");
        assert!(
            report.failed.is_empty(),
            "{first} then {second}: {report:?}"
        );
        let all = r.list(&Default::default()).await.unwrap();
        assert_eq!(all.len(), 1, "{first} then {second}");
        assert_eq!(all[0].envelope.name, "First", "{first} then {second}");
    }
}

#[tokio::test]
async fn item_1_get_by_uid_finds_any_spelling_and_two_plain_uids_stay_apart() {
    let r = repo().await;
    let stored = r
        .create(card_with_uid(&format!("urn:uuid:{V7}"), "A"))
        .await
        .unwrap();
    let found = r.get_by_uid(&V7.to_uppercase()).await.unwrap().unwrap();
    assert_eq!(found.envelope.id, stored.envelope.id);
    r.create(card_with_uid("plain-one", "B")).await.unwrap();
    r.create(card_with_uid("plain-two", "C")).await.unwrap();
    assert_eq!(r.list(&Default::default()).await.unwrap().len(), 3);
}

#[test]
fn item_2_normalising_twice_changes_nothing() {
    assert_eq!(
        normalise_phone("00"),
        normalise_phone(&normalise_phone("00"))
    );
    assert_eq!(normalise_phone("0049 30"), "+4930");
    for mail in ["mailto:mailto:a@x.org", "mailto: mailto: A@x.org"] {
        let once = normalise_email(mail);
        assert_eq!(normalise_email(&once), once, "{mail}");
        assert_eq!(once, "a@x.org");
    }
}

fn unclear_markings() -> Vec<serde_json::Value> {
    vec![
        json!(true),
        json!([]),
        json!({"pd:X": true}),
        json!(""),
        json!(null),
        json!([1]),
    ]
}

#[tokio::test]
async fn item_4_a_new_card_with_an_unclear_marking_is_refused() {
    let r = repo().await;
    for marking in unclear_markings() {
        let mut card = person("New", "n@x.org");
        card.extra
            .insert("freesynergy.net:sensitive".into(), marking.clone());
        let got = r.create(card).await;
        assert!(matches!(got, Err(Error::Invalid(_))), "{marking}: {got:?}");
    }
    assert!(r.list(&Default::default()).await.unwrap().is_empty());
    let mut nested = person("Nested", "n@x.org");
    let entry = nested.emails.values_mut().next().unwrap();
    entry
        .extra
        .insert("freesynergy.net:sensitive".into(), json!(true));
    assert!(matches!(r.create(nested).await, Err(Error::Invalid(_))));
}

#[tokio::test]
async fn item_4_an_import_keeps_a_card_with_an_unclear_marking_names_it_and_treats_it_as_sensitive()
{
    for marking in unclear_markings() {
        let r = repo().await;
        let mut card = person("Unclear", "u@x.org");
        card.extra
            .insert("freesynergy.net:sensitive".into(), marking.clone());
        let text = serde_json::to_string(&card).unwrap();
        let report = r.import_jscontact(&text).await.unwrap();
        assert_eq!(report.created.len(), 1, "{marking}: {report:?}");
        assert_eq!(report.unclear.len(), 1, "{marking}: {report:?}");
        assert!(report.unclear[0].contains("Unclear"), "{report:?}");
        let id = report.created[0];
        let stored = r.get_card(id).await.unwrap().unwrap();
        assert!(stored.is_sensitive(), "{marking}");
        assert_eq!(
            stored.extra["freesynergy.net:sensitive"], marking,
            "kept as it came"
        );
        let object = r.get(id).await.unwrap().unwrap();
        assert!(
            !object.envelope.sensitive.is_empty(),
            "{marking}: the store must treat it so"
        );
        assert!(r.find_same_person_proposals().await.unwrap().is_empty());
        // Still editable: an update is not new input.
        let mut edited = stored;
        edited.add_phone("+49 30 1");
        r.update(id, edited).await.unwrap();
    }
}

#[tokio::test]
async fn item_4_a_plain_store_reports_an_unclear_marking_as_failed_with_its_reason() {
    let dir = std::env::temp_dir().join(format!("fs-review-{}", ObjectId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    let store: Arc<dyn Store> = Arc::new(TursoStore::open(dir.join("c.db")).await.unwrap());
    let r = PeopleRepo::new(store, ME);
    let mut card = person("Unclear", "u@x.org");
    card.extra
        .insert("freesynergy.net:sensitive".into(), json!(true));
    let text = serde_json::to_string(&vec![card, person("Fine", "f@x.org")]).unwrap();
    let report = r.import_jscontact(&text).await.unwrap();
    assert_eq!(report.created.len(), 1, "{report:?}");
    assert_eq!(report.failed.len(), 1, "{report:?}");
    assert!(report.failed[0].contains("Unclear"), "{report:?}");
    assert!(report.failed[0].contains("not encrypted"), "{report:?}");
    std::fs::remove_dir_all(dir).ok();
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

/// A real store whose deletes fail on request.
struct Failing {
    store: Arc<dyn Store>,
    fail_cards: AtomicBool,
    fail_others: AtomicBool,
}

#[async_trait]
impl ObjectRepository for Failing {
    async fn put(&self, object: StoredObject, actor: &str) -> fs_store::Result<Activity> {
        Store::put(&*self.store, object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> fs_store::Result<Vec<Activity>> {
        Store::put_and_log(&*self.store, object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> fs_store::Result<Option<StoredObject>> {
        Store::get(&*self.store, id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> fs_store::Result<Activity> {
        let is_card = Store::get(&*self.store, id)
            .await?
            .is_some_and(|o| o.envelope.object_type == "Profile");
        let flag = if is_card {
            &self.fail_cards
        } else {
            &self.fail_others
        };
        if flag.load(Ordering::Relaxed) {
            return Err(fs_store::Error::Invalid("the disk is full".into()));
        }
        Store::delete(&*self.store, id, actor).await
    }
}

#[async_trait]
impl ObjectQuery for Failing {
    async fn query(&self, query: &Query) -> fs_store::Result<Vec<StoredObject>> {
        Store::query(&*self.store, query).await
    }
    async fn count(&self, query: &Query) -> fs_store::Result<usize> {
        Store::count(&*self.store, query).await
    }
}

async fn with_failing_store() -> (PeopleRepo, Arc<Failing>) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let failing = Arc::new(Failing {
        store,
        fail_cards: AtomicBool::new(false),
        fail_others: AtomicBool::new(false),
    });
    let repo = PeopleRepo::with_backend(failing.clone(), failing.clone(), ME);
    (repo, failing)
}

#[tokio::test]
async fn item_14_a_refused_delete_of_the_card_leaves_links_and_roles_in_place() {
    let (r, failing) = with_failing_store().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let b = r
        .create(person("Bob", "b@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.confirm_same_person(a, b).await.unwrap();
    r.mark_ai(a).await.unwrap();
    failing.fail_cards.store(true, Ordering::Relaxed);
    assert!(r.delete(a).await.is_err());
    assert!(r.get(a).await.unwrap().is_some());
    assert_eq!(
        r.linked_to(a).await.unwrap(),
        [b],
        "the link is still there"
    );
    assert!(r.is_ai(a).await.unwrap(), "the role is still there");
}

#[tokio::test]
async fn item_14_a_failed_clean_up_is_said_and_a_second_delete_finishes_it() {
    let (r, failing) = with_failing_store().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let b = r
        .create(person("Bob", "b@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.confirm_same_person(a, b).await.unwrap();
    failing.fail_others.store(true, Ordering::Relaxed);
    assert!(r.delete(a).await.is_err(), "the left-overs are reported");
    assert!(r.get(a).await.unwrap().is_none());
    failing.fail_others.store(false, Ordering::Relaxed);
    r.delete(a).await.unwrap();
    assert!(r.linked_to(b).await.unwrap().is_empty());
    assert!(
        matches!(r.delete(a).await, Err(Error::NotFound(_))),
        "nothing left to do"
    );
}
