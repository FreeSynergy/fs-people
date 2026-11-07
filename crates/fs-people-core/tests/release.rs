//! A sensitive card leaves the program only with a release, also one marked in its content alone.

mod common;

use common::{person, repo};
use fs_objects::{Envelope, StoredObject};
use fs_people_core::{Card, Error, JsContactFormat, VCardFormat};

fn marked(name: &str, email: &str, categories: &[&str]) -> Card {
    let mut c = person(name, email);
    c.mark_sensitive(categories.iter().copied());
    c
}

#[tokio::test]
async fn a_sensitive_card_leaves_only_with_a_release() {
    let r = repo().await;
    let id = r
        .create(marked("Secret", "s@x.org", &["pd:HealthRecord"]))
        .await
        .unwrap()
        .envelope
        .id;
    let uid = r.get_card(id).await.unwrap().unwrap().uid;
    let refused = r.export(&VCardFormat, id).await.unwrap_err();
    assert!(
        matches!(&refused, Error::NotReleased(_, who) if who.contains("Secret")),
        "{refused}"
    );
    assert!(matches!(
        r.export_released(&JsContactFormat, id, &["other".to_string()])
            .await,
        Err(Error::NotReleased(..))
    ));
    let upper = uid.to_uppercase();
    let released = r
        .export_released(&JsContactFormat, id, &[upper])
        .await
        .unwrap();
    assert!(released.text.contains("s@x.org"), "{}", released.text);
}

#[tokio::test]
async fn an_older_mark_in_the_content_alone_counts_at_export_too() {
    let (r, store) = common::setup().await;
    let member = marked("Old", "o@x.org", &["pd:HealthRecord"]);
    let uid = member.uid.clone();
    let id = r
        .create(person("Plain", "p@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    // Stored before the envelope carried it: only the content knows.
    let mut older = StoredObject::new(
        Envelope::new("Profile", "Old"),
        "application/jscontact+json",
        member.to_value(),
    )
    .with_index("uid", uid.as_str())
    .with_index("kind", "individual");
    older.envelope.context = Some("people".into());
    let older_id = older.envelope.id;
    store.put(older, common::ME).await.unwrap();
    let list = r.create_group("L").await.unwrap().envelope.id;
    r.add_member(list, id).await.unwrap();
    let mut card = r.get_card(list).await.unwrap().unwrap();
    card.members.insert(uid.clone(), true);
    r.update(list, card).await.unwrap();

    let without = r
        .export_released(&JsContactFormat, list, &[])
        .await
        .unwrap();
    assert_eq!(without.withheld, [format!("{uid}: Old")]);
    assert!(!without.text.contains(&uid), "{}", without.text);
    assert!(matches!(
        r.export(&JsContactFormat, older_id).await,
        Err(Error::NotReleased(..))
    ));
}
