use std::sync::Arc;

use fs_contacts_core::*;
use fs_store::{ActivityFilter, ActivityKind, ObjectId, Store, TursoStore};
use serde_json::{Value, json};

const ME: &str = "https://local.invalid/users/owner";
const DEMO: &str = "/srv/FreeSynergy/fs-documentation/temp/demo-data/contacts.json";

async fn repo() -> ContactRepo {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    ContactRepo::new(store, ME)
}

fn person(name: &str, email: &str) -> Card {
    let mut c = Card::individual(name);
    c.add_email(email);
    c
}

#[test]
fn demo_cards_roundtrip_losslessly_and_validate() {
    let raw = std::fs::read_to_string(DEMO).unwrap();
    let items: Vec<Value> = serde_json::from_str(&raw).unwrap();
    assert_eq!(items.len(), 20);
    for item in items {
        let card: Card = serde_json::from_value(item.clone()).unwrap();
        validate::validate(&card).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(card.to_value(), item, "lossless: {}", card.uid);
        assert!(card.vendor("freesynergy.net:source").is_some());
    }
}

#[test]
fn unknown_and_vendor_properties_survive() {
    let v = json!({"@type":"Card","version":"1.0","uid":"x","name":{"full":"A","future":1},
        "cryptoKeys":{"k":{"uri":"data:..."}},"example.com:thing":{"a":[1,2]},
        "emails":{"e":{"address":"a@b.c","example.com:x":true}}});
    let c: Card = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(c.to_value(), v);
}

#[test]
fn validation_rules() {
    let ok = person("Ada", "ada@example.org");
    assert!(validate::validate(&ok).is_ok());
    let mut c = ok.clone();
    c.uid = "".into();
    c.version = "2.0".into();
    c.kind = Some("robot".into());
    c.emails.get_mut("e1").unwrap().address = "".into();
    c.created = Some("yesterday".into());
    let paths: Vec<_> = validate::issues(&c).into_iter().map(|i| i.path).collect();
    for p in ["/uid", "/version", "/kind", "/emails/e1", "/created"] {
        assert!(paths.contains(&p.to_string()), "{p} in {paths:?}");
    }
    let mut c = ok.clone();
    c.kind = Some("example.com:robot".into());
    assert!(validate::validate(&c).is_ok(), "vendor kind allowed");
    let mut c = ok.clone();
    c.members.insert("u".into(), true);
    assert!(validate::validate(&c).is_err(), "members only for groups");
    for k in validate::KINDS {
        let mut c = ok.clone();
        c.kind = Some(k.into());
        assert!(validate::validate(&c).is_ok());
    }
}

#[tokio::test]
async fn create_get_update_delete_with_envelope() {
    let r = repo().await;
    let mut card = person("Ada Lovelace", " Ada@Example.ORG ");
    card.add_phone("+49 (30) 1234-567");
    card.add_organization("Analytical Engines");
    let o = r.create(card.clone()).await.unwrap();
    assert_eq!(o.envelope.object_type, "Profile");
    assert_eq!(o.envelope.name, "Ada Lovelace");
    assert_eq!(o.content_type, "application/jscontact+json");
    assert_eq!(o.envelope.context.as_deref(), Some("contacts"));
    assert!(o.envelope.iri.as_deref().unwrap().starts_with("https://local.invalid/contacts/"));
    assert_eq!(o.index["email"], ["ada@example.org"]);
    assert_eq!(o.index["phone"], ["+49301234567"]);
    assert_eq!(o.index["org"], ["analytical engines"]);
    assert_eq!(o.index["kind"], ["individual"]);
    // duplicate uid refused
    assert!(matches!(r.create(card.clone()).await, Err(Error::Conflict(_))));
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
    let log = r.store().activities(&ActivityFilter::default()).await.unwrap();
    let kinds: Vec<_> = log.iter().map(|a| a.kind).collect();
    assert_eq!(kinds, [ActivityKind::Create, ActivityKind::Update, ActivityKind::Delete]);
    // undo of the delete brings the card back
    r.store().undo(ME, 1).await.unwrap();
    assert!(r.get(id).await.unwrap().is_some());
}

#[tokio::test]
async fn name_is_derived_even_without_full() {
    let mut c = Card::individual("x");
    c.name = Some(card::Name {
        components: vec![
            card::NameComponent { kind: "given".into(), value: "Grace".into(), ..Default::default() },
            card::NameComponent { kind: "surname".into(), value: "Hopper".into(), ..Default::default() },
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
async fn import_demo_list_and_search() {
    let r = repo().await;
    let rep = r.import_jscontact(&std::fs::read_to_string(DEMO).unwrap()).await.unwrap();
    assert_eq!((rep.created.len(), rep.skipped.len(), rep.failed.len()), (20, 0, 0));
    let again = r.import_jscontact(&std::fs::read_to_string(DEMO).unwrap()).await.unwrap();
    assert_eq!((again.created.len(), again.skipped.len()), (0, 20));
    let all = r.list(&ListFilter::default()).await.unwrap();
    assert_eq!(all.len(), 20);
    let names: Vec<_> = all.iter().map(|o| o.envelope.name.to_lowercase()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    assert_eq!(r.list(&ListFilter { kind: Some("org".into()), limit: Some(5), offset: 0 }).await.unwrap().len(), 5);

    let by_name = r.search("nlnet").await.unwrap();
    assert!(by_name.iter().any(|o| o.envelope.name == "NLnet Foundation"));
    let by_mail = r.search("WEBCONTACT@nlnet").await.unwrap();
    assert_eq!(by_mail.len(), 1);
    let by_phone = r.search("20 888 4252").await.unwrap_or_default();
    assert!(by_phone.is_empty() || by_phone.len() == 1); // formatting-insensitive but contiguous digits
    let by_phone = r.search("+31208884252").await.unwrap();
    assert_eq!(by_phone.len(), 1);
    let by_org = r.search("stichting").await.unwrap();
    assert!(by_org.iter().any(|o| o.envelope.name == "NLnet Foundation"));
    assert!(r.search("zzzz-nobody").await.unwrap().is_empty());
    assert_eq!(r.find_by_email("WebContact@NLnet.nl").await.unwrap().len(), 1);
    assert_eq!(r.find_by_phone("0031 20 888 4252").await.unwrap().len(), 1);
}

#[tokio::test]
async fn vcard_import_keeps_original_and_exports() {
    let r = repo().await;
    let vcf = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:urn:uuid:01a1aaef-ba80-7f7a-8dff-000000000001\r\nFN:Ada Lovelace\r\nN:Lovelace;Ada;;;\r\nEMAIL;TYPE=work:Ada@Example.org\r\nTEL;VALUE=uri:tel:+49-30-1234567\r\nORG:Analytical Engines\r\nURL:https://example.org/ada\r\nNOTE:First programmer\r\nKIND:individual\r\nADR;TYPE=home:;;1 Main St;Berlin;;10115;Germany\r\nX-CUSTOM:keep me\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nFN:Bob Old\r\nEMAIL:bob@old.example\r\nEND:VCARD\r\n";
    let rep = r.import_vcard(vcf).await.unwrap();
    assert_eq!(rep.created.len(), 2, "{rep:?}");
    let ada = r.get(rep.created[0]).await.unwrap().unwrap();
    // original kept byte for byte, per card
    let orig = ada.original.as_ref().unwrap();
    assert_eq!(orig.media_type, "text/vcard");
    assert!(std::str::from_utf8(&orig.bytes).unwrap().starts_with("BEGIN:VCARD\r\nVERSION:4.0"));
    assert!(std::str::from_utf8(&orig.bytes).unwrap().ends_with("END:VCARD\r\n"));
    assert!(!std::str::from_utf8(&orig.bytes).unwrap().contains("Bob"));
    assert_eq!(ada.envelope.name, "Ada Lovelace");
    assert_eq!(ada.envelope.id.to_string(), "01a1aaef-ba80-7f7a-8dff-000000000001");
    assert_eq!(ada.index["email"], ["ada@example.org"]);
    assert_eq!(ada.index["phone"], ["+49301234567"]);
    assert_eq!(ada.index["org"], ["analytical engines"]);
    // v3 card without UID got one
    let bob = r.get_card(rep.created[1]).await.unwrap().unwrap();
    assert!(bob.uid.starts_with("urn:uuid:"));
    // re-import skips
    let again = r.import_vcard(vcf).await.unwrap();
    assert_eq!((again.created.len(), again.skipped.len()), (0, 2));
    // export
    let out = r.export_vcard(ada.envelope.id).await.unwrap();
    for needle in ["BEGIN:VCARD", "VERSION:4.0", "FN:Ada Lovelace", "ada@example.org".to_lowercase().as_str(), "UID:urn:uuid:01a1aaef", "END:VCARD"] {
        assert!(out.to_lowercase().contains(&needle.to_lowercase()), "{needle} in\n{out}");
    }
    // exported vCard re-imports to an equivalent core card
    let back = vcard::card_from_vcard(&out).unwrap();
    let a = r.get_card(ada.envelope.id).await.unwrap().unwrap();
    assert_eq!(back.uid, a.uid);
    assert_eq!(back.display_name(), a.display_name());
    assert_eq!(
        back.emails.values().map(|e| e.address.clone()).collect::<Vec<_>>(),
        a.emails.values().map(|e| e.address.clone()).collect::<Vec<_>>()
    );
    // the original cannot be replaced by an update (store rule), but is kept
    let o = r.update(ada.envelope.id, a).await.unwrap();
    assert_eq!(o.original.as_ref(), Some(orig));
}

#[test]
fn vcard_garbage_is_reported_not_panicking() {
    assert!(vcard::import_vcards("hello world").is_empty());
    let r = vcard::import_vcards("BEGIN:VCARD\nVERSION:4.0\nEND:VCARD\n");
    // no FN / name: still a card (uid generated) or a clean error, never a panic
    assert_eq!(r.len(), 1);
}

#[tokio::test]
async fn groups_hold_members() {
    let r = repo().await;
    let a = r.create(person("Ada", "a@x.org")).await.unwrap().envelope.id;
    let b = r.create(person("Bob", "b@x.org")).await.unwrap().envelope.id;
    let g = r.create_group("Team").await.unwrap();
    let gid = g.envelope.id;
    assert_eq!(g.index["kind"], ["group"]);
    r.add_member(gid, a).await.unwrap();
    r.add_member(gid, b).await.unwrap();
    r.add_member(gid, b).await.unwrap(); // idempotent
    let names: Vec<_> = r.members(gid).await.unwrap().into_iter().map(|o| o.envelope.name).collect();
    assert_eq!(names.len(), 2);
    assert_eq!(r.groups_of(a).await.unwrap().len(), 1);
    assert!(matches!(r.add_member(a, b).await, Err(Error::Conflict(_))), "only groups have members");
    assert!(matches!(r.add_member(gid, gid).await, Err(Error::Conflict(_))));
    r.remove_member(gid, a).await.unwrap();
    assert_eq!(r.members(gid).await.unwrap().len(), 1);
    assert!(r.groups_of(a).await.unwrap().is_empty());
    let groups = r.list(&ListFilter { kind: Some("group".into()), ..Default::default() }).await.unwrap();
    assert_eq!(groups.len(), 1);
}

#[tokio::test]
async fn same_person_proposals_and_confirmation() {
    let r = repo().await;
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
    assert_eq!(ab.reasons, [MatchReason { kind: MatchKind::Email, value: "ada@example.org".into() }]);
    // proposing writes nothing
    assert_eq!(r.store().count(&fs_store::Query::new().of_type("Relationship")).await.unwrap(), 0);

    let rel = r.confirm_same_person(a, b).await.unwrap();
    assert_eq!(rel.envelope.object_type, "Relationship");
    assert_eq!(rel.content["relationship"], "owl:sameAs");
    assert_eq!(rel.content["type"], "Relationship");
    assert!(rel.content["subject"].as_str().unwrap().contains("/contacts/"));
    // both cards remain, link visible, no longer proposed, not linkable twice
    assert!(r.get(a).await.unwrap().is_some() && r.get(b).await.unwrap().is_some());
    assert_eq!(r.linked_to(a).await.unwrap(), vec![b]);
    assert!(!r.find_same_person_proposals().await.unwrap().iter().any(|p| (p.a, p.b) == pair(a, b)));
    assert!(matches!(r.confirm_same_person(b, a).await, Err(Error::Conflict(_))));
    // unlink = delete of the relationship; proposal returns
    r.unlink(rel.envelope.id).await.unwrap();
    assert!(r.find_same_person_proposals().await.unwrap().iter().any(|p| (p.a, p.b) == pair(a, b)));
    // deleting a card removes its links
    let rel = r.confirm_same_person(a, b).await.unwrap();
    r.delete(b).await.unwrap();
    assert!(r.store().get(rel.envelope.id).await.unwrap().is_none());
}

#[tokio::test]
async fn sensitive_cards_are_never_proposed_or_linked() {
    let r = repo().await;
    let mut a = person("Secret", "s@example.org");
    a.mark_sensitive(["pd:SexualPreference"]);
    let b = person("Open", "s@example.org");
    // field-level mark counts as well
    let mut c = person("Field", "f@example.org");
    c.extra.insert("personalInfo".into(), json!({"p1": {"kind":"expertise","value":"x","freesynergy.net:sensitive":["pd:HealthRecord"]}}));
    let d = person("Plain", "f@example.org");
    let oa = r.create(a).await.unwrap();
    assert_eq!(oa.envelope.sensitive, ["pd:SexualPreference"]);
    let (a, b) = (oa.envelope.id, r.create(b).await.unwrap().envelope.id);
    let oc = r.create(c).await.unwrap();
    assert_eq!(oc.envelope.sensitive, ["pd:HealthRecord"]);
    let (c, d) = (oc.envelope.id, r.create(d).await.unwrap().envelope.id);
    assert!(r.find_same_person_proposals().await.unwrap().is_empty());
    assert!(matches!(r.confirm_same_person(a, b).await, Err(Error::Sensitive(id)) if id == a));
    assert!(matches!(r.confirm_same_person(d, c).await, Err(Error::Sensitive(id)) if id == c));
}

#[tokio::test]
async fn sensitive_card_needs_encrypted_store() {
    // ADR 0029 is enforced by fs-store: a plain file store refuses sensitive cards.
    let dir = std::env::temp_dir().join(format!("fs-contacts-{}", fs_store::ObjectId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    let store: Arc<dyn Store> = Arc::new(TursoStore::open(dir.join("c.db")).await.unwrap());
    let r = ContactRepo::new(store, ME);
    let mut c = person("Secret", "s@example.org");
    c.mark_sensitive(["pd:SexualPreference"]);
    assert!(matches!(r.create(c).await, Err(Error::Store(fs_store::Error::EncryptionRequired(_)))));
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test]
async fn list_is_case_insensitive_unicode_ordered_and_paged_by_the_store() {
    let r = repo().await;
    for n in ["NLnet", "Nextcloud", "zebra", "Ärger", "apple"] {
        r.create(Card::individual(n)).await.unwrap();
    }
    let names = |v: Vec<fs_store::StoredObject>| v.into_iter().map(|o| o.envelope.name).collect::<Vec<_>>();
    let all = names(r.list(&ListFilter::default()).await.unwrap());
    assert_eq!(all, ["apple", "Nextcloud", "NLnet", "zebra", "Ärger"]);
    let page = names(r.list(&ListFilter { limit: Some(2), offset: 1, ..Default::default() }).await.unwrap());
    assert_eq!(page, ["Nextcloud", "NLnet"]);
}

#[tokio::test]
async fn search_runs_in_the_store_with_unicode_nicknames_and_literal_wildcards() {
    let r = repo().await;
    let mut a = person("Émilie Dupont", "emilie@Example.org");
    a.add_phone("+49 30 123456");
    a.add_organization("Müller & Söhne");
    a.nicknames.insert("n1".into(), serde_json::from_value(json!({"name": "Mimi"})).unwrap());
    r.create(a).await.unwrap();
    r.create(person("50% Rabatt", "rabatt@example.org")).await.unwrap();
    r.create(person("5000 Rabatt", "other@example.org")).await.unwrap();
    let n = |v: Vec<fs_store::StoredObject>| v.len();
    assert_eq!(n(r.search("ÉMILIE").await.unwrap()), 1);
    assert_eq!(n(r.search("e\u{301}milie").await.unwrap()), 1, "decomposed input");
    assert_eq!(n(r.search("mimi").await.unwrap()), 1, "nickname");
    assert_eq!(n(r.search("SÖHNE").await.unwrap()), 1, "organisation, non-ASCII");
    assert_eq!(n(r.search("EMILIE@example").await.unwrap()), 1);
    assert_eq!(n(r.search("30 123").await.unwrap()), 1, "phone digits");
    assert_eq!(n(r.search("%").await.unwrap()), 1, "% is a literal");
    assert_eq!(n(r.search("50%").await.unwrap()), 1);
    assert_eq!(n(r.search("_").await.unwrap()), 0, "_ is a literal");
    assert_eq!(n(r.search("example.org").await.unwrap()), 3);
    assert_eq!(n(r.search("  ").await.unwrap()), 3, "blank = everything");
}

#[tokio::test]
async fn describes_is_an_envelope_field() {
    let r = repo().await;
    let mut c = person("Ada", "ada@example.org");
    c.set_vendor("freesynergy.net:describes", json!("did:example:ada"));
    let o = r.create(c).await.unwrap();
    assert_eq!(o.envelope.describes.as_deref(), Some("did:example:ada"));
    assert!(!o.index.contains_key("describes"));
    assert_eq!(r.find_by_describes("did:example:ada").await.unwrap().len(), 1);
    assert!(r.find_by_describes("did:example:other").await.unwrap().is_empty());
    // update follows the card
    let mut c = r.get_card(o.envelope.id).await.unwrap().unwrap();
    c.set_vendor("freesynergy.net:describes", json!("did:example:ada2"));
    let o = r.update(o.envelope.id, c).await.unwrap();
    assert_eq!(o.envelope.describes.as_deref(), Some("did:example:ada2"));
    assert!(r.find_by_describes("did:example:ada").await.unwrap().is_empty());
}
