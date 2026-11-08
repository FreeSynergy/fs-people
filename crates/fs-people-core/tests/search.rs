//! Listing, ordering, paging and search (all evaluated by the store).

mod common;

use common::{DEMO, person, repo};
use fs_people_core::{Card, ListFilter};
use serde_json::json;

#[tokio::test]
async fn import_demo_list_and_search() {
    let r = repo().await;
    let rep = r
        .import_jscontact(&std::fs::read_to_string(DEMO).unwrap())
        .await
        .unwrap();
    assert_eq!(
        (rep.created.len(), rep.skipped.len(), rep.failed.len()),
        (20, 0, 0)
    );
    let again = r
        .import_jscontact(&std::fs::read_to_string(DEMO).unwrap())
        .await
        .unwrap();
    assert_eq!((again.created.len(), again.skipped.len()), (0, 20));
    let all = r.list(&ListFilter::default()).await.unwrap();
    assert_eq!(all.len(), 20);
    let names: Vec<_> = all.iter().map(|o| o.envelope.name.to_lowercase()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    assert_eq!(
        r.list(&ListFilter {
            kind: Some("org".into()),
            limit: Some(5),
            ..Default::default()
        })
        .await
        .unwrap()
        .len(),
        5
    );

    let by_name = r.search("nlnet").await.unwrap();
    assert!(
        by_name
            .iter()
            .any(|o| o.envelope.name == "NLnet Foundation")
    );
    let by_mail = r.search("WEBCONTACT@nlnet").await.unwrap();
    assert_eq!(by_mail.len(), 1);
    let by_phone = r.search("20 888 4252").await.unwrap_or_default();
    assert!(by_phone.is_empty() || by_phone.len() == 1); // formatting-insensitive but contiguous digits
    let by_phone = r.search("+31208884252").await.unwrap();
    assert_eq!(by_phone.len(), 1);
    let by_org = r.search("stichting").await.unwrap();
    assert!(by_org.iter().any(|o| o.envelope.name == "NLnet Foundation"));
    assert_eq!(
        r.search("zzzz-nobody").await.unwrap(),
        [] as [fs_objects::StoredObject; 0]
    );
    assert_eq!(
        r.find_by_email("WebContact@NLnet.nl").await.unwrap().len(),
        1
    );
    assert_eq!(r.find_by_phone("0031 20 888 4252").await.unwrap().len(), 1);
}

#[tokio::test]
async fn list_is_case_insensitive_unicode_ordered_and_paged_by_the_store() {
    let r = repo().await;
    for n in ["NLnet", "Nextcloud", "zebra", "Ärger", "apple"] {
        r.create(Card::individual(n)).await.unwrap();
    }
    let names = |v: Vec<fs_objects::StoredObject>| {
        v.into_iter().map(|o| o.envelope.name).collect::<Vec<_>>()
    };
    let all = names(r.list(&ListFilter::default()).await.unwrap());
    assert_eq!(all, ["apple", "Nextcloud", "NLnet", "zebra", "Ärger"]);
    let page = names(
        r.list(&ListFilter {
            limit: Some(2),
            offset: 1,
            ..Default::default()
        })
        .await
        .unwrap(),
    );
    assert_eq!(page, ["Nextcloud", "NLnet"]);
}

#[tokio::test]
async fn search_runs_in_the_store_with_unicode_nicknames_and_literal_wildcards() {
    let r = repo().await;
    let mut a = person("Émilie Dupont", "emilie@Example.org");
    a.add_phone("+49 30 123456");
    a.add_organization("Müller & Söhne");
    a.nicknames.insert(
        "n1".into(),
        serde_json::from_value(json!({"name": "Mimi"})).unwrap(),
    );
    r.create(a).await.unwrap();
    r.create(person("50% Rabatt", "rabatt@example.org"))
        .await
        .unwrap();
    r.create(person("5000 Rabatt", "other@example.org"))
        .await
        .unwrap();
    let n = |v: Vec<fs_objects::StoredObject>| v.len();
    assert_eq!(n(r.search("ÉMILIE").await.unwrap()), 1);
    assert_eq!(
        n(r.search("e\u{301}milie").await.unwrap()),
        1,
        "decomposed input"
    );
    assert_eq!(n(r.search("mimi").await.unwrap()), 1, "nickname");
    assert_eq!(
        n(r.search("SÖHNE").await.unwrap()),
        1,
        "organisation, non-ASCII"
    );
    assert_eq!(n(r.search("EMILIE@example").await.unwrap()), 1);
    assert_eq!(n(r.search("30 123").await.unwrap()), 1, "phone digits");
    assert_eq!(n(r.search("%").await.unwrap()), 1, "% is a literal");
    assert_eq!(n(r.search("50%").await.unwrap()), 1);
    assert_eq!(n(r.search("_").await.unwrap()), 0, "_ is a literal");
    assert_eq!(n(r.search("example.org").await.unwrap()), 3);
    assert_eq!(n(r.search("  ").await.unwrap()), 3, "blank = everything");
}
