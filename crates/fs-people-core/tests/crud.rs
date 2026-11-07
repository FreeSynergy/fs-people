//! Create / read / update / delete, envelope derivation and undo through the store.

mod common;

use common::{ME, person, repo, setup};
use fs_objects::{ActivityFilter, ActivityKind};
use fs_people_core::{Card, Error, card};
use serde_json::json;

#[tokio::test]
async fn create_get_update_delete_with_envelope() {
    let (r, store) = setup().await;
    let mut card = person("Ada Lovelace", " Ada@Example.ORG ");
    card.add_phone("+49 (30) 1234-567");
    card.add_organization("Analytical Engines");
    let o = r.create(card.clone()).await.unwrap();
    assert_eq!(o.envelope.object_type, "Profile");
    assert_eq!(o.envelope.name, "Ada Lovelace");
    assert_eq!(o.content_type, "application/jscontact+json");
    assert_eq!(o.envelope.context.as_deref(), Some("people"));
    assert!(
        o.envelope
            .iri
            .as_deref()
            .unwrap()
            .starts_with("https://local.invalid/people/")
    );
    assert_eq!(o.index["email"], ["ada@example.org"]);
    assert_eq!(o.index["phone"], ["+49301234567"]);
    assert_eq!(o.index["org"], ["analytical engines"]);
    assert_eq!(o.index["kind"], ["individual"]);
    // duplicate uid refused
    assert!(matches!(
        r.create(card.clone()).await,
        Err(Error::Conflict(_))
    ));
    // invalid refused
    let mut bad = person("X", "x@y.z");
    bad.version.clear();
    assert!(matches!(r.create(bad).await, Err(Error::Invalid(_))));

    let id = o.envelope.id;
    let mut c = r.get_card(id).await.unwrap().unwrap();
    c.name.as_mut().unwrap().full = Some("Ada King".into());
    c.emails.clear();
    c.add_email("ada@king.example");
    let o2 = r.update(id, c.clone()).await.unwrap();
    assert_eq!(o2.envelope.name, "Ada King");
    assert_eq!(o2.index["email"], ["ada@king.example"]);
    assert_eq!(o2.envelope.published, o.envelope.published);
    // uid is immutable
    c.uid = "urn:uuid:other".into();
    assert!(matches!(r.update(id, c).await, Err(Error::Conflict(_))));

    r.delete(id).await.unwrap();
    assert!(r.get(id).await.unwrap().is_none());
    assert!(matches!(r.delete(id).await, Err(Error::NotFound(_))));
    let log = store.activities(&ActivityFilter::default()).await.unwrap();
    let kinds: Vec<_> = log.iter().map(|a| a.kind).collect();
    assert_eq!(
        kinds,
        [
            ActivityKind::Create,
            ActivityKind::Update,
            ActivityKind::Delete
        ]
    );
    // undo of the delete brings the card back
    store.undo(ME, 1).await.unwrap();
    assert!(r.get(id).await.unwrap().is_some());
}

#[tokio::test]
async fn name_is_derived_even_without_full() {
    let mut c = Card::individual("x");
    c.name = Some(card::Name {
        components: vec![
            card::NameComponent {
                kind: "given".into(),
                value: "Grace".into(),
                ..Default::default()
            },
            card::NameComponent {
                kind: "surname".into(),
                value: "Hopper".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    assert_eq!(c.display_name(), "Grace Hopper");
    let o = repo().await.create(c).await.unwrap();
    assert_eq!(o.envelope.name, "Grace Hopper");
    assert_eq!(o.index["name_sort"], ["hopper, grace"]);
    let mut anon = Card::individual("x");
    anon.name = None;
    anon.add_email("only@mail.example");
    assert_eq!(anon.display_name(), "only@mail.example");
}

#[tokio::test]
async fn describes_is_an_envelope_field() {
    let r = repo().await;
    let mut c = person("Ada", "ada@example.org");
    c.set_vendor("freesynergy.net:describes", json!("did:example:ada"));
    let o = r.create(c).await.unwrap();
    assert_eq!(o.envelope.describes.as_deref(), Some("did:example:ada"));
    assert!(!o.index.contains_key("describes"));
    assert_eq!(
        r.find_by_describes("did:example:ada").await.unwrap().len(),
        1
    );
    assert!(
        r.find_by_describes("did:example:other")
            .await
            .unwrap()
            .is_empty()
    );
    // update follows the card
    let mut c = r.get_card(o.envelope.id).await.unwrap().unwrap();
    c.set_vendor("freesynergy.net:describes", json!("did:example:ada2"));
    let o = r.update(o.envelope.id, c).await.unwrap();
    assert_eq!(o.envelope.describes.as_deref(), Some("did:example:ada2"));
    assert!(
        r.find_by_describes("did:example:ada")
            .await
            .unwrap()
            .is_empty()
    );
}
