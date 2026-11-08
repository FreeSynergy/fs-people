//! An edit never reduces a sensitive marking, an unclear one stays unclear, and a mere mention marks
//! nobody.
//!
//! Pattern: Integration tests over the real core and the real state machine.

mod common;

use common::{World, person};
use fs_people_core::Card;
use serde_json::json;

const KEY: &str = "freesynergy.net:sensitive";

fn error_of(s: &fs_components_workbenches::people::PeopleState) -> String {
    s.editing()
        .and_then(|e| e.error.clone())
        .unwrap_or_default()
}

fn with_marked_name_part() -> Card {
    serde_json::from_value(json!({
        "@type": "Card", "version": "1.0", "uid": "urn:uuid:01a1aaef-ba80-7f7a-8dff-9f85769beae4",
        "name": {"components": [
            {"kind": "given", "value": "Ada", KEY: ["pd:HealthRecord"]},
            {"kind": "surname", "value": "L"}
        ]},
        "emails": {"e1": {"address": "a@x.org", KEY: ["pd:Religion"]}, "e2": {"address": "b@x.org"}}
    }))
    .unwrap()
}

#[test]
fn item_3_a_rename_never_reduces_the_sensitive_categories() {
    let w = World::in_memory();
    let key = w.add(with_marked_name_part());
    let before = w.card(&key).sensitive_categories();
    assert_eq!(before, ["pd:HealthRecord", "pd:Religion"]);
    let mut s = w.state();
    s.select(&key);
    s.begin_edit();
    s.set_field("name/full", "Augusta");
    s.save();
    assert!(s.editing().is_none(), "{}", error_of(&s));
    let card = w.card(&key);
    assert_eq!(
        card.name.as_ref().unwrap().components,
        [] as [fs_people_core::card::NameComponent; 0]
    );
    assert_eq!(card.sensitive_categories(), before);
    let stored = w
        .runtime
        .block_on(w.repo.get(key.parse().unwrap()))
        .unwrap()
        .unwrap();
    assert_eq!(stored.envelope.sensitive, before, "the store knows as well");
}

#[test]
fn item_3_removing_the_marked_e_mail_keeps_its_category_and_the_person_stays_out_of_an_export() {
    let (w, dir) = (World::in_memory(), common::Scratch::new());
    let key = w.add(with_marked_name_part());
    let mut s = w.state();
    s.select(&key);
    s.begin_edit();
    s.set_field("emails/e1", "");
    s.save();
    let card = w.card(&key);
    assert!(!card.emails.contains_key("e1"));
    assert!(
        card.sensitive_categories()
            .contains(&"pd:Religion".to_string())
    );
    let out = dir.file("out.vcf");
    let report = fs_people_app::export_all(w.repo.as_ref(), w.runtime.handle(), &out, &[]).unwrap();
    assert_eq!(report.written, 0);
    assert_eq!(report.left_out.len(), 1);
}

#[test]
fn a_marking_without_categories_counts_as_sensitive_on_export_and_is_marked() {
    let (w, dir) = (World::in_memory(), common::Scratch::new());
    for (i, marking) in [json!(true), json!([]), json!({"pd:X": true})]
        .iter()
        .enumerate()
    {
        let mut card = person(&format!("Unclear {i}"), "u@x.org");
        card.extra.insert(KEY.into(), marking.clone());
        // An import keeps such a card on either core; `create` may refuse it.
        let text = serde_json::to_string(&card).unwrap();
        w.runtime.block_on(w.repo.import_jscontact(&text)).unwrap();
    }
    w.add(person("Open", "o@x.org"));
    let out = dir.file("out.vcf");
    let report = fs_people_app::export_all(w.repo.as_ref(), w.runtime.handle(), &out, &[]).unwrap();
    assert_eq!(report.written, 1, "{report:?}");
    assert_eq!(report.left_out.len(), 3, "{report:?}");
}

#[test]
fn item_f_an_unclear_marking_moved_to_card_level_stays_unclear() {
    let w = World::in_memory();
    let card: Card = serde_json::from_value(json!({
        "@type": "Card", "version": "1.0", "uid": "urn:uuid:01a1aaef-ba80-7f7a-8dff-9f85769beae5",
        "name": {"full": "Ada"},
        "emails": {"e1": {"address": "a@x.org", KEY: true}, "e2": {"address": "b@x.org"}}
    }))
    .unwrap();
    let text = serde_json::to_string(&card).unwrap();
    let report = w.runtime.block_on(w.repo.import_jscontact(&text)).unwrap();
    let key = report.created[0].to_string();
    let mut s = w.state();
    s.select(&key);
    s.begin_edit();
    s.set_field("emails/e1", "");
    s.save();
    assert!(s.editing().is_none(), "{}", error_of(&s));
    let stored = w.card(&key);
    assert!(!stored.emails.contains_key("e1"));
    assert_eq!(stored.extra[KEY], json!(true), "as it came, not a category");
    assert!(
        stored.sensitive_categories().is_empty(),
        "no category invented"
    );
}

#[test]
fn item_f_beside_named_categories_an_unclear_marking_still_reads_as_unclear() {
    let w = World::in_memory();
    let card: Card = serde_json::from_value(json!({
        "@type": "Card", "version": "1.0", "uid": "urn:uuid:01a1aaef-ba80-7f7a-8dff-9f85769beae6",
        "name": {"full": "Ada"},
        KEY: ["pd:Religion"],
        "emails": {"e1": {"address": "a@x.org", KEY: true}, "e2": {"address": "b@x.org"}}
    }))
    .unwrap();
    let text = serde_json::to_string(&card).unwrap();
    let report = w.runtime.block_on(w.repo.import_jscontact(&text)).unwrap();
    let key = report.created[0].to_string();
    let mut s = w.state();
    s.select(&key);
    s.begin_edit();
    s.set_field("emails/e1", "");
    s.save();
    assert_eq!(w.card(&key).extra[KEY], json!(["pd:Religion", true]));
}

#[test]
fn item_h_a_note_that_mentions_the_property_marks_nobody() {
    let (w, dir) = (World::in_memory(), common::Scratch::new());
    let mut card = person("Curious", "c@x.org");
    card.notes.insert(
        "n1".into(),
        serde_json::from_value(json!({"note": format!("what is {KEY}?")})).unwrap(),
    );
    let key = w.add(card);
    let s = w.state();
    assert!(s.rows()[0].marks.is_empty(), "not marked");
    assert_eq!(s.selected(), Some(key.as_str()), "and opened like anyone");
    let out = dir.file("out.vcf");
    let report = fs_people_app::export_all(w.repo.as_ref(), w.runtime.handle(), &out, &[]).unwrap();
    assert_eq!(report.written, 1, "{report:?}");
}
