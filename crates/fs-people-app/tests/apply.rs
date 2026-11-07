//! Findings of the review about writing a form back: freed ids, an emptied name, an empty card,
//! and releasing a person for export. The marking itself is in `marks.rs`.
//!
//! Integration tests over the real core and the real state machine.

mod common;

use common::{World, person};
use serde_json::json;

fn error_of(s: &fs_components_workbenches::people::PeopleState) -> String {
    s.editing()
        .and_then(|e| e.error.clone())
        .unwrap_or_default()
}

#[test]
fn item_9_a_freed_organisation_id_is_not_reused_and_the_title_lets_go_of_it() {
    let w = World::in_memory();
    let mut card = person("Ada", "a@x.org");
    let org = card.add_organization("Engines");
    card.titles.insert(
        "t1".into(),
        serde_json::from_value(json!({"name": "Analyst", "organizationId": org})).unwrap(),
    );
    let key = w.add(card);
    let mut s = w.state();
    s.begin_edit();
    s.set_field(&format!("organizations/{org}"), "");
    s.add_field("organizations");
    let added = s
        .editing()
        .unwrap()
        .draft
        .fields()
        .find(|f| f.added)
        .unwrap()
        .key
        .clone();
    s.set_field(&added, "Looms");
    s.save();
    assert!(s.editing().is_none(), "{}", error_of(&s));
    let card = w.card(&key);
    assert!(
        !card.organizations.contains_key(&org),
        "the id of this edit is not reused"
    );
    assert_eq!(
        card.organizations.values().next().unwrap().name.as_deref(),
        Some("Looms")
    );
    assert_eq!(
        card.titles["t1"].organization_id, None,
        "not pointing at anything"
    );
    assert_eq!(card.titles["t1"].name, "Analyst");
}

#[test]
fn item_9_a_title_keeps_an_organisation_that_stays() {
    let w = World::in_memory();
    let mut card = person("Ada", "a@x.org");
    let org = card.add_organization("Engines");
    card.titles.insert(
        "t1".into(),
        serde_json::from_value(json!({"name": "Analyst", "organizationId": org})).unwrap(),
    );
    let key = w.add(card);
    let mut s = w.state();
    s.begin_edit();
    s.set_field(&format!("organizations/{org}"), "Engines Ltd");
    s.save();
    assert_eq!(
        w.card(&key).titles["t1"].organization_id.as_deref(),
        Some(org.as_str())
    );
}

#[test]
fn item_11_a_name_with_sort_as_can_be_emptied_and_more_than_that_says_why_not() {
    let w = World::in_memory();
    let mut card = person("Ada", "a@x.org");
    card.name
        .as_mut()
        .unwrap()
        .extra
        .insert("sortAs".into(), json!("ADA"));
    let key = w.add(card);
    let mut s = w.state();
    s.begin_edit();
    s.set_field("name/full", "");
    s.save();
    assert!(s.editing().is_none(), "{}", error_of(&s));
    assert!(w.card(&key).name.is_none());

    let w = World::in_memory();
    let mut card = person("Bob", "b@x.org");
    card.name
        .as_mut()
        .unwrap()
        .extra
        .insert("example.org:x".into(), json!(1));
    let key = w.add(card);
    let mut s = w.state();
    s.begin_edit();
    s.set_field("name/full", "");
    s.save();
    assert!(error_of(&s).contains("example.org:x"), "{}", error_of(&s));
    assert_eq!(w.card(&key).display_name(), "Bob", "nothing was written");
}

#[test]
fn item_12_save_cannot_empty_what_create_refuses() {
    let w = World::in_memory();
    let key = w.add(person("Ada", "a@x.org"));
    let mut s = w.state();
    s.begin_edit();
    s.set_field("name/full", "");
    s.set_field("emails/e1", "");
    s.save();
    assert!(error_of(&s).contains("at least"), "{}", error_of(&s));
    assert_eq!(w.card(&key).display_name(), "Ada");
}

#[test]
fn export_sensitive_releases_exactly_the_person_named() {
    let (w, dir) = (World::in_memory(), common::Scratch::new());
    let mut one = person("Secret One", "one@x.org");
    one.mark_sensitive(["pd:HealthRecord"]);
    let one_uid = one.uid.clone();
    let mut two = person("Secret Two", "two@x.org");
    two.mark_sensitive(["pd:HealthRecord"]);
    w.add(one);
    w.add(two);
    w.add(person("Open", "open@x.org"));
    let out = dir.file("out.vcf");
    let released = vec![one_uid];
    let report =
        fs_people_app::export_all(w.people.as_ref(), w.runtime.handle(), &out, &released).unwrap();
    assert_eq!(report.written, 2);
    assert_eq!(report.released.len(), 1);
    assert_eq!(report.left_out.len(), 1);
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("one@x.org") && text.contains("open@x.org"));
    assert!(!text.contains("two@x.org"));
    assert!(report.message(&out).contains("Released by you"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&out).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "for its owner alone");
    }
}

#[test]
fn export_sensitive_naming_nobody_writes_nothing() {
    let (w, dir) = (World::in_memory(), common::Scratch::new());
    w.add(person("Open", "open@x.org"));
    let out = dir.file("out.vcf");
    let released = vec!["urn:uuid:nobody".to_string()];
    let got = fs_people_app::export_all(w.people.as_ref(), w.runtime.handle(), &out, &released);
    assert!(got.unwrap_err().contains("names nobody"));
    assert!(!out.exists());
}

#[test]
fn item_12_an_organisation_of_units_alone_is_not_something_to_call_a_person_by() {
    let w = World::in_memory();
    let mut card = person("Ada", "a@x.org");
    let org = card.add_organization("Engines");
    let entry = card.organizations.get_mut(&org).unwrap();
    entry.units = vec![json!({"name": "Maths"})];
    let key = w.add(card);
    let mut s = w.state();
    s.begin_edit();
    s.set_field("name/full", "");
    s.set_field("emails/e1", "");
    s.set_field(&format!("organizations/{org}"), "");
    s.save();
    assert!(error_of(&s).contains("at least"), "{}", error_of(&s));
    assert_eq!(w.card(&key).display_name(), "Ada");
}
