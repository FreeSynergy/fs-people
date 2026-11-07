//! The cards through the trait `People`: the client against the real server and the real store.
//!
//! Integration tests against the trait; no stand-ins.

mod common;

use common::{Served, add, person};
use fs_people_core::{Error, ListFilter, People};
use fs_store::ObjectId;

fn names(found: &[fs_store::StoredObject]) -> Vec<&str> {
    found.iter().map(|o| o.envelope.name.as_str()).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_card_goes_in_comes_back_changes_and_goes() {
    let served = Served::start().await;
    let people = served.client();
    let created = people.create(person("Ada", "ada@x.org")).await.unwrap();
    assert_eq!(created.envelope.name, "Ada");
    assert_eq!(created.content_type, "application/jscontact+json");
    assert_eq!(created.content["emails"]["e1"]["address"], "ada@x.org");
    assert_eq!(created.envelope.context.as_deref(), Some("people"));

    let id = created.envelope.id;
    let got = people.get(id).await.unwrap().unwrap();
    assert_eq!(got.content, created.content);
    assert_eq!(got.envelope.id, id);

    let mut card = fs_people_core::card_of(&got).unwrap();
    card.add_email("ada@y.org");
    let changed = people.update(id, card).await.unwrap();
    assert_eq!(changed.content["emails"].as_object().unwrap().len(), 2);
    assert!(changed.envelope.updated >= created.envelope.updated);

    people.delete(id).await.unwrap();
    assert!(people.get(id).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_list_is_in_name_order_and_honours_limit_and_offset() {
    let served = Served::start().await;
    let people = served.client();
    for name in ["Cid", "ada", "Bob"] {
        add(&people, person(name, &format!("{name}@x.org"))).await;
    }
    let all = people.list(&ListFilter::default()).await.unwrap();
    assert_eq!(names(&all), ["ada", "Bob", "Cid"]);
    let filter = ListFilter {
        limit: Some(1),
        offset: 1,
        ..ListFilter::default()
    };
    assert_eq!(names(&people.list(&filter).await.unwrap()), ["Bob"]);
    let kind = ListFilter {
        kind: Some("group".into()),
        ..ListFilter::default()
    };
    assert!(people.list(&kind).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn every_page_is_read_not_only_the_first() {
    let served = Served::start().await;
    let people = served.client();
    for i in 0..230 {
        add(&people, person(&format!("P{i:03}"), &format!("p{i}@x.org"))).await;
    }
    let all = people.list(&ListFilter::default()).await.unwrap();
    assert_eq!(all.len(), 230, "the wire pages hold 100");
    assert_eq!(all[229].envelope.name, "P229");
    let found = people.search("@x.org").await.unwrap();
    assert_eq!(found.len(), 230);
    let tail = ListFilter {
        offset: 225,
        ..ListFilter::default()
    };
    assert_eq!(people.list(&tail).await.unwrap().len(), 5);
}

#[tokio::test(flavor = "multi_thread")]
async fn search_finds_by_name_mail_phone_and_organisation() {
    let served = Served::start().await;
    let people = served.client();
    let mut ada = person("Ada", "ada@x.org");
    ada.add_phone("+49 30 123456");
    ada.add_organization("Engines Ltd");
    add(&people, ada).await;
    add(&people, person("Bob", "bob@y.org")).await;
    for (term, who) in [
        ("ada", "Ada"),
        ("y.org", "Bob"),
        ("123456", "Ada"),
        ("engines", "Ada"),
    ] {
        assert_eq!(names(&people.search(term).await.unwrap()), [who], "{term}");
    }
    assert!(people.search("nothing").await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn what_the_server_refuses_comes_back_as_the_matching_error() {
    let served = Served::start().await;
    let people = served.client();
    let nobody = ObjectId::new();
    assert!(matches!(
        people.update(nobody, person("X", "x@x.org")).await,
        Err(Error::NotFound(id)) if id == nobody
    ));
    assert!(matches!(
        people.delete(nobody).await,
        Err(Error::NotFound(_))
    ));

    let first = person("Ada", "a@x.org");
    people.create(first.clone()).await.unwrap();
    let again = people.create(first).await;
    assert!(matches!(again, Err(Error::Conflict(_))), "{again:?}");

    let mut bad = person("Bad", "b@x.org");
    bad.version = "9.9".into();
    let refused = people.create(bad).await.unwrap_err();
    assert!(
        matches!(refused, Error::Invalid(ref issues) if !issues.is_empty()),
        "{refused:?}"
    );
    assert!(
        refused.to_string().starts_with("card violates RFC 9553: "),
        "{refused}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plain_store_refusing_a_sensitive_card_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let served = Served::start_with_file(&dir.path().join("p.db")).await;
    let people = served.client();
    let mut secret = person("Secret", "s@x.org");
    secret.mark_sensitive(["pd:SexualPreference"]);
    let refused = people.create(secret).await.unwrap_err();
    assert!(
        matches!(
            refused,
            Error::Store(fs_store::Error::EncryptionRequired(_))
        ),
        "{refused:?}"
    );
    assert!(refused.to_string().contains("not encrypted"), "{refused}");
}

#[tokio::test(flavor = "multi_thread")]
async fn capabilities_promise_no_more_than_the_client_keeps() {
    let served = Served::start().await;
    let people = served.client();
    let can = people.capabilities();
    for id in [
        "person.create",
        "person.update",
        "person.delete",
        "person.read",
        "person.find",
        "person.import",
        "person.export",
        "role.read",
    ] {
        assert!(can.is_supported(id), "{id}");
    }
    // What the server offers and the client does not: not possible, with a reason that says so.
    for id in [
        "person.subscribe",
        "relation.write",
        "cardgroup.write",
        "same.link",
        "role.assign",
        "ai.mark",
    ] {
        let why = can
            .why_not(id)
            .unwrap_or_else(|| panic!("{id} is promised"));
        assert!(why.reason().contains("client"), "{id}: {}", why.reason());
    }
    // And the methods agree with the report.
    let refused = people.create_group("G").await.unwrap_err();
    assert!(matches!(refused, Error::Unsupported(_)));
}
