//! Same-person proposals, links and the sensitive rules (ADR 0019, 0029).

mod common;

use std::sync::Arc;

use common::{ME, person, repo, setup};
use fs_objects::{ObjectId, Store, TursoStore};
use fs_people_core::{Error, MatchKind, MatchReason, PeopleRepo};
use serde_json::json;

#[allow(clippy::many_single_char_names)] // short fixture names (repo, person, group)
#[tokio::test]
async fn same_person_proposals_and_confirmation() {
    let (r, store) = setup().await;
    let mut a = person("Ada L.", "ada@example.org");
    a.add_phone("+49 30 1234567");
    let mut b = person("Ada Lovelace", "ADA@example.org");
    b.add_phone("030 1234567"); // different national format: no match
    let mut c = person("Countess", "other@example.org");
    c.add_phone("0049 30 1234567"); // matches a by phone
    let d = person("Unrelated", "d@example.org");
    let (a, b, c, d) = (
        r.create(a).await.unwrap().envelope.id,
        r.create(b).await.unwrap().envelope.id,
        r.create(c).await.unwrap().envelope.id,
        r.create(d).await.unwrap().envelope.id,
    );
    let props = r.find_same_person_proposals().await.unwrap();
    let pair = |x: ObjectId, y: ObjectId| if x < y { (x, y) } else { (y, x) };
    let found: Vec<_> = props.iter().map(|p| (p.a, p.b)).collect();
    assert!(found.contains(&pair(a, b)), "email match");
    assert!(found.contains(&pair(a, c)), "phone match");
    assert!(!found.contains(&pair(b, c)));
    assert!(found.iter().all(|(x, y)| *x != d && *y != d));
    assert!(props.iter().all(|p| p.a < p.b));
    let ab = props.iter().find(|p| (p.a, p.b) == pair(a, b)).unwrap();
    assert_eq!(
        ab.reasons,
        [MatchReason {
            kind: MatchKind::Email,
            value: "ada@example.org".into()
        }]
    );
    // proposing writes nothing
    assert_eq!(
        store
            .count(&fs_objects::Query::new().of_type("Relationship"))
            .await
            .unwrap(),
        0
    );

    let rel = r.confirm_same_person(a, b).await.unwrap();
    assert_eq!(rel.envelope.object_type, "Relationship");
    assert_eq!(rel.content["relationship"], "owl:sameAs");
    assert_eq!(rel.content["type"], "Relationship");
    assert!(
        rel.content["subject"]
            .as_str()
            .unwrap()
            .contains("/people/")
    );
    // both cards remain, link visible, no longer proposed, not linkable twice
    assert!(r.get(a).await.unwrap().is_some() && r.get(b).await.unwrap().is_some());
    assert_eq!(r.linked_to(a).await.unwrap(), vec![b]);
    assert!(
        !r.find_same_person_proposals()
            .await
            .unwrap()
            .iter()
            .any(|p| (p.a, p.b) == pair(a, b))
    );
    assert!(matches!(
        r.confirm_same_person(b, a).await,
        Err(Error::Conflict(_))
    ));
    // unlink = delete of the relationship; the pair then counts as rejected (ADR 0058), so the
    // proposal does not return until the rejection is taken back
    r.unlink(rel.envelope.id).await.unwrap();
    assert!(
        !r.find_same_person_proposals()
            .await
            .unwrap()
            .iter()
            .any(|p| (p.a, p.b) == pair(a, b))
    );
    // deleting a card removes its links
    let rel = r.confirm_same_person(a, b).await.unwrap();
    r.delete(b).await.unwrap();
    assert!(store.get(rel.envelope.id).await.unwrap().is_none());
}

#[allow(clippy::many_single_char_names)] // short fixture names (repo, person, group)
#[tokio::test]
async fn sensitive_cards_are_never_proposed_or_linked() {
    let r = repo().await;
    let mut a = person("Secret", "s@example.org");
    a.mark_sensitive(["pd:SexualPreference"]);
    let b = person("Open", "s@example.org");
    // field-level mark counts as well
    let mut c = person("Field", "f@example.org");
    c.personal_info.insert(
        "p1".into(),
        serde_json::from_value(
            json!({"kind":"expertise","value":"x","freesynergy.net:sensitive":["pd:HealthRecord"]}),
        )
        .unwrap(),
    );
    let d = person("Plain", "f@example.org");
    let oa = r.create(a).await.unwrap();
    assert_eq!(oa.envelope.sensitive, ["pd:SexualPreference"]);
    let (a, b) = (oa.envelope.id, r.create(b).await.unwrap().envelope.id);
    let oc = r.create(c).await.unwrap();
    assert_eq!(oc.envelope.sensitive, ["pd:HealthRecord"]);
    let (c, d) = (oc.envelope.id, r.create(d).await.unwrap().envelope.id);
    assert_eq!(
        r.find_same_person_proposals().await.unwrap(),
        [] as [fs_people_core::SameAsProposal; 0]
    );
    assert!(matches!(r.confirm_same_person(a, b).await, Err(Error::Sensitive(id)) if id == a));
    assert!(matches!(r.confirm_same_person(d, c).await, Err(Error::Sensitive(id)) if id == c));
}

#[tokio::test]
async fn sensitive_card_needs_encrypted_store() {
    // ADR 0029 is enforced by fs-objects: a plain file store refuses sensitive cards.
    let dir = std::env::temp_dir().join(format!("fs-contacts-{}", fs_objects::ObjectId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    let store: Arc<dyn Store> = Arc::new(TursoStore::open(dir.join("c.db")).await.unwrap());
    let r = PeopleRepo::new(store, ME);
    let mut c = person("Secret", "s@example.org");
    c.mark_sensitive(["pd:SexualPreference"]);
    assert!(matches!(
        r.create(c).await,
        Err(Error::Store(fs_objects::Error::EncryptionRequired(_)))
    ));
    std::fs::remove_dir_all(dir).ok();
}
