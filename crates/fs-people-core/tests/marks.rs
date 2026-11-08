//! An edit never takes a sensitive marking away, a card that becomes sensitive loses its links, and a
//! list with sensitive members does not take them out of the program.

mod common;

use common::{person, repo};
use fs_people_core::card::SENSITIVE_KEY;
use fs_people_core::{Card, Error, JsContactFormat, VCardFormat};
use serde_json::json;

fn marked(name: &str, email: &str, categories: &[&str]) -> Card {
    let mut c = person(name, email);
    c.mark_sensitive(categories.iter().copied());
    c
}

#[tokio::test]
async fn an_update_without_the_mark_keeps_it() {
    let r = repo().await;
    let a = r
        .create(marked("Ada", "a@x.org", &["pd:HealthRecord"]))
        .await
        .unwrap()
        .envelope
        .id;
    r.create(person("A. K.", "a@x.org")).await.unwrap();
    let mut bare = person("Ada", "a@x.org");
    bare.uid = r.get_card(a).await.unwrap().unwrap().uid;
    let stored = r.update(a, bare.clone()).await.unwrap();
    assert_eq!(stored.envelope.sensitive, ["pd:HealthRecord"]);
    assert_eq!(
        r.find_same_person_proposals().await.unwrap(),
        [] as [fs_people_core::SameAsProposal; 0]
    );

    // Another named category adds to it, and removes nothing.
    bare.mark_sensitive(["pd:Religion"]);
    let stored = r.update(a, bare).await.unwrap();
    assert_eq!(
        stored.envelope.sensitive,
        ["pd:HealthRecord", "pd:Religion"]
    );
}

#[tokio::test]
async fn an_unclear_mark_stays_until_categories_replace_it() {
    let r = repo().await;
    let text = json!({"@type": "Card", "version": "1.0", "uid": "u", "name": {"full": "Ada"},
        SENSITIVE_KEY: true});
    let report = r.import_jscontact(&text.to_string()).await.unwrap();
    let id = report.created[0];
    assert_eq!(report.unclear.len(), 1);

    let mut bare = Card::individual("Ada");
    bare.uid = "u".into();
    let stored = r.update(id, bare.clone()).await.unwrap();
    assert_eq!(
        stored.envelope.sensitive,
        [fs_people_core::card::UNCLEAR_SENSITIVE]
    );
    assert_eq!(
        stored.content[SENSITIVE_KEY],
        json!(true),
        "stays what it was"
    );

    bare.mark_sensitive(["pd:HealthRecord"]);
    let stored = r.update(id, bare).await.unwrap();
    assert_eq!(stored.envelope.sensitive, ["pd:HealthRecord"]);
    assert_eq!(stored.content[SENSITIVE_KEY], json!(["pd:HealthRecord"]));
}

#[tokio::test]
async fn a_card_that_becomes_sensitive_loses_its_links() {
    let r = repo().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let b = r
        .create(person("A. K.", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.confirm_same_person(a, b).await.unwrap();
    let mut card = r.get_card(a).await.unwrap().unwrap();
    card.mark_sensitive(["pd:HealthRecord"]);
    r.update(a, card).await.unwrap();
    assert_eq!(
        r.linked_to(a).await.unwrap(),
        [] as [fs_objects::ObjectId; 0]
    );
    assert_eq!(
        r.linked_to(b).await.unwrap(),
        [] as [fs_objects::ObjectId; 0]
    );
}

#[tokio::test]
async fn a_sensitive_member_can_be_added_on_request_but_does_not_leave_with_the_list() {
    let r = repo().await;
    let secret = r
        .create(marked("Secret", "s@x.org", &["pd:HealthRecord"]))
        .await
        .unwrap();
    let secret_uid = r.get_card(secret.envelope.id).await.unwrap().unwrap().uid;
    let plain = r
        .create(person("Plain", "p@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let plain_uid = r.get_card(plain).await.unwrap().unwrap().uid;
    let list = r.create_group("L").await.unwrap().envelope.id;
    r.add_member(list, secret.envelope.id).await.unwrap();
    r.add_member(list, plain).await.unwrap();

    // The mark travels with the member.
    let members = r.members(list).await.unwrap();
    let carried = members
        .iter()
        .find(|o| o.envelope.id == secret.envelope.id)
        .unwrap();
    assert_eq!(carried.envelope.sensitive, ["pd:HealthRecord"]);

    let refused = r.export(&JsContactFormat, list).await.unwrap_err();
    assert!(matches!(&refused, Error::SensitiveMembers(_, names) if names[0].contains("Secret")));

    let without = r
        .export_released(&JsContactFormat, list, &[])
        .await
        .unwrap();
    assert!(!without.text.contains(&secret_uid), "{}", without.text);
    assert!(without.text.contains(&plain_uid), "{}", without.text);
    assert_eq!(without.withheld, [format!("{secret_uid}: Secret")]);

    let released = r
        .export_released(&VCardFormat, list, std::slice::from_ref(&secret_uid))
        .await
        .unwrap();
    assert!(released.text.contains(&secret_uid), "{}", released.text);
    assert_eq!(released.withheld, [] as [std::string::String; 0]);

    // The list itself did not change.
    assert_eq!(r.members(list).await.unwrap().len(), 2);
}
