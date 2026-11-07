//! Editing keeps everything else on the card, through the real core and the real state machine.
//!
//! Pattern: Round-trip tests — the card as JSON before and after, compared but for what was edited.

mod common;

use common::{World, person};
use fs_people_core::card::{Name, NameComponent};
use fs_people_core::{Card, People};
use serde_json::{Value, json};

/// A card with something in every corner the form does not show.
fn rich() -> Card {
    let mut card: Card = serde_json::from_value(json!({
        "@type": "Card",
        "version": "1.0",
        "uid": "urn:uuid:01a1aaef-ba80-7f7a-8dff-9f85769beae3",
        "kind": "individual",
        "name": {
            "@type": "Name",
            "components": [
                {"@type": "NameComponent", "kind": "given", "value": "Ada"},
                {"@type": "NameComponent", "kind": "surname", "value": "Lovelace"}
            ],
            "isOrdered": true,
            "example.org:phonetic": "ay-da"
        },
        "nicknames": {"n1": {"@type": "Nickname", "name": "Countess"}},
        "titles": {"t1": {"@type": "Title", "name": "Analyst", "organizationId": "o1"}},
        "emails": {
            "e1": {"@type": "EmailAddress", "address": "ada@x.org", "contexts": {"work": true},
                   "pref": 1, "label": "desk", "example.org:verified": true},
            "e2": {"@type": "EmailAddress", "address": "home@x.org", "contexts": {"private": true}}
        },
        "phones": {"p1": {"@type": "Phone", "number": "+49 30 1", "features": {"voice": true},
                          "contexts": {"work": true}}},
        "organizations": {
            "o1": {"@type": "Organization", "name": "Engines", "sortAs": "ENGINES",
                   "units": [{"@type": "OrgUnit", "name": "Maths"}]}
        },
        "addresses": {"a1": {"@type": "Address", "full": "12 St James", "countryCode": "GB"}},
        "links": {"l1": {"@type": "Link", "uri": "https://x.org"}},
        "notes": {"no1": {"@type": "Note", "note": "first programmer"}},
        "keywords": {"maths": true},
        "relatedTo": {"urn:uuid:other": {"relation": {"friend": true}}},
        "personalInfo": {"pi1": {"@type": "PersonalInfo", "kind": "expertise", "value": "maths"}},
        "cryptoKeys": {"k1": {"@type": "CryptoKey", "uri": "https://x.org/key"}},
        "freesynergy.net:source": "https://x.org/ada",
        "example.org:anything": {"deep": [1, 2, {"three": true}]}
    }))
    .unwrap();
    card.created = Some("2026-11-05T09:00:00Z".into());
    card.updated = Some("2026-11-05T09:00:00Z".into());
    card
}

/// The card as JSON without the time of the last change.
fn json_of(card: &Card) -> Value {
    let mut value = card.to_value();
    value.as_object_mut().unwrap().remove("updated");
    value
}

#[test]
fn editing_one_phone_changes_that_number_and_nothing_else() {
    let w = World::in_memory();
    let key = w.add(rich());
    let before = json_of(&w.card(&key));
    let mut s = w.state();
    s.begin_edit();
    s.set_field("phones/p1", "+49 30 2");
    s.save();
    assert_eq!(
        s.editing().map(|e| e.error.clone()),
        None,
        "{:?}",
        s.notice()
    );
    let mut expected = before;
    expected["phones"]["p1"]["number"] = json!("+49 30 2");
    assert_eq!(json_of(&w.card(&key)), expected);
}

#[test]
fn a_changed_e_mail_keeps_its_contexts_and_preference_and_the_other_stays() {
    let w = World::in_memory();
    let key = w.add(rich());
    let before = json_of(&w.card(&key));
    let mut s = w.state();
    s.begin_edit();
    s.set_field("emails/e1", "lady@x.org");
    s.save();
    let mut expected = before;
    expected["emails"]["e1"]["address"] = json!("lady@x.org");
    assert_eq!(json_of(&w.card(&key)), expected);
}

#[test]
fn an_emptied_e_mail_goes_alone_and_an_added_one_takes_a_free_id() {
    let w = World::in_memory();
    let key = w.add(rich());
    let mut s = w.state();
    s.begin_edit();
    s.set_field("emails/e2", "   ");
    s.add_field("emails");
    let added = s
        .editing()
        .unwrap()
        .draft
        .fields()
        .find(|f| f.added)
        .unwrap()
        .key
        .clone();
    s.set_field(&added, "new@x.org");
    s.save();
    let card = w.card(&key);
    assert!(card.emails.contains_key("e1"));
    assert!(
        !card.emails.contains_key("e2"),
        "a freed id is not reused in the same edit"
    );
    assert!(!card.emails.values().any(|e| e.address == "home@x.org"));
    assert_eq!(card.emails["e1"].label.as_deref(), Some("desk"));
    assert!(card.emails.values().any(|e| e.address == "new@x.org"));
    assert_eq!(card.emails.len(), 2);
}

#[test]
fn a_new_name_replaces_the_parts_and_sorting_and_search_follow_and_the_rest_stays() {
    let w = World::in_memory();
    let key = w.add(rich());
    let mut s = w.state();
    s.begin_edit();
    let shown = s
        .editing()
        .unwrap()
        .draft
        .fields()
        .next()
        .unwrap()
        .value
        .clone();
    assert_eq!(shown, "Ada Lovelace", "the parts are shown as the name");
    s.set_field("name/full", "Augusta King");
    s.save();
    let card = w.card(&key);
    let name = card.name.as_ref().unwrap();
    assert_eq!(name.full.as_deref(), Some("Augusta King"));
    assert!(name.components.is_empty(), "the old name must not live on");
    assert_eq!(name.extra["example.org:phonetic"], json!("ay-da"));
    assert_eq!(
        card.extra["example.org:anything"]["deep"][2]["three"],
        json!(true)
    );
    s.search("Lovelace");
    assert!(s.rows().is_empty(), "the old name no longer finds her");
    s.search("King");
    assert_eq!(s.rows().len(), 1);
}

#[test]
fn an_unchanged_name_of_parts_stays_parts() {
    let w = World::in_memory();
    let key = w.add(rich());
    let mut s = w.state();
    s.begin_edit();
    s.set_field("phones/p1", "+49 30 9");
    s.save();
    let name: Name = w.card(&key).name.unwrap();
    assert_eq!(name.full, None, "no name.full was invented");
    let parts: Vec<&NameComponent> = name.components.iter().collect();
    assert_eq!(parts.len(), 2);
}

#[test]
fn an_organisation_with_units_only_loses_its_name_when_emptied() {
    let w = World::in_memory();
    let key = w.add(rich());
    let mut s = w.state();
    s.begin_edit();
    s.set_field("organizations/o1", "");
    s.save();
    assert!(
        s.editing().is_none(),
        "{:?}",
        s.editing().map(|e| e.error.clone())
    );
    let organisation = &w.card(&key).organizations["o1"];
    assert_eq!(organisation.name, None);
    assert_eq!(organisation.units.len(), 1);
    assert_eq!(organisation.sort_as.as_deref(), Some("ENGINES"));
}

#[test]
fn the_imported_original_survives_an_edit() {
    let w = World::in_memory();
    let vcard =
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Grace Hopper\r\nEMAIL:grace@x.org\r\nEND:VCARD\r\n";
    w.runtime.block_on(w.people.import_vcard(vcard)).unwrap();
    let mut s = w.state();
    let key = s.selected().unwrap().to_string();
    s.begin_edit();
    s.set_field("name/full", "Rear Admiral Hopper");
    s.save();
    let id = key.parse().unwrap();
    // The original does not travel over the wire, so it is read where it is kept: in the store
    // behind the server. The edit itself went through the client.
    let stored = w.runtime.block_on(w.repo.get(id)).unwrap().unwrap();
    let original = stored.original.expect("ADR 0030: the original is kept");
    assert_eq!(String::from_utf8(original.bytes).unwrap(), vcard);
}

#[test]
fn roles_and_the_mark_of_an_ai_survive_an_edit_and_are_shown_read_only() {
    let w = World::in_memory();
    let key = w.add(person("Helper", "bot@x.org"));
    let id = key.parse().unwrap();
    w.runtime.block_on(w.repo.mark_ai(id)).unwrap();
    let mut s = w.state();
    let sheet = |s: &fs_components_workbenches::people::PeopleState| match s.shown() {
        fs_components_workbenches::people::Shown::Person { sheet, .. } => sheet.clone(),
        other => panic!("not a person: {other:?}"),
    };
    assert_eq!(sheet(&s).marks, ["AI"]);
    assert!(sheet(&s).sections.iter().any(|sec| sec.title == "Roles"));
    s.begin_edit();
    let fields: Vec<String> = s
        .editing()
        .unwrap()
        .draft
        .fields()
        .map(|f| f.key.clone())
        .collect();
    assert!(
        !fields
            .iter()
            .any(|f| f.contains("role") || f.contains("kind")),
        "read only"
    );
    s.set_field("emails/e1", "helper@x.org");
    s.save();
    assert!(w.runtime.block_on(w.repo.is_ai(id)).unwrap());
    assert_eq!(w.card(&key).kind(), "application");
    assert_eq!(sheet(&s).marks, ["AI"]);
}

#[test]
fn a_change_by_someone_else_is_a_conflict_and_writing_knowingly_keeps_their_other_change() {
    let w = World::in_memory();
    let key = w.add(person("Ada", "ada@x.org"));
    let id = key.parse().unwrap();
    let mut s = w.state();
    s.begin_edit();
    s.set_field("name/full", "Mine");

    let mut theirs = w.card(&key);
    theirs.add_phone("+49 30 7");
    w.runtime.block_on(w.people.update(id, theirs)).unwrap();

    s.save();
    let edit = s.editing().expect("the draft stays");
    assert!(edit.error.as_deref().unwrap().contains("Someone else"));
    assert_eq!(w.card(&key).display_name(), "Ada", "nothing was written");

    s.save();
    assert!(s.editing().is_none());
    let card = w.card(&key);
    assert_eq!(card.display_name(), "Mine");
    assert_eq!(card.phones.len(), 1, "their phone is still there");
}
