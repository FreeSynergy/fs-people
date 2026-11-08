//! List, search, read, create, delete and what the provider can do, through the real core.
//!
//! Pattern: Integration tests over the adapter and the state machine; no window.

mod common;

use common::{World, person};
use fs_components_workbenches::ErrorKind;
use fs_components_workbenches::people::{PeopleSource, Shown};
use fs_objects::ObjectId;
use fs_people_core::Card;

fn names(world: &World, term: &str) -> Vec<String> {
    world
        .adapter
        .rows(term, 0, 1000)
        .unwrap()
        .into_iter()
        .map(|r| r.cells[0].clone())
        .collect()
}

#[test]
fn the_list_is_in_name_order_and_paged_by_offset_and_limit() {
    let w = World::in_memory();
    for name in ["Cid", "ada", "Bob"] {
        w.add(person(name, &format!("{name}@x.org")));
    }
    assert_eq!(names(&w, ""), ["ada", "Bob", "Cid"]);
    let page = w.adapter.rows("", 1, 1).unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].cells, ["Bob", "Bob@x.org"]);
}

#[test]
fn search_finds_by_name_mail_phone_and_organisation_and_pages_too() {
    let w = World::in_memory();
    let mut ada = person("Ada", "ada@x.org");
    ada.add_phone("+49 30 123456");
    ada.add_organization("Engines Ltd");
    w.add(ada);
    w.add(person("Bob", "bob@y.org"));
    assert_eq!(names(&w, "ada"), ["Ada"]);
    assert_eq!(names(&w, "y.org"), ["Bob"]);
    assert_eq!(names(&w, "123456"), ["Ada"]);
    assert_eq!(names(&w, "engines"), ["Ada"]);
    assert!(names(&w, "nothing").is_empty());
    assert_eq!(w.adapter.rows("org", 1, 5).unwrap().len(), 1);
}

#[test]
fn more_people_than_a_page_are_reached_through_the_state_machine() {
    let w = World::in_memory();
    for i in 0..205 {
        w.add(Card::individual(format!("P{i:03}")));
    }
    let mut s = w.state();
    assert_eq!(s.rows().len(), 200);
    assert!(s.has_more());
    s.load_more();
    assert_eq!(s.rows().len(), 205);
    assert!(!s.has_more());
}

#[test]
fn a_person_is_read_as_sections_and_what_is_not_shown_is_named() {
    let w = World::in_memory();
    let mut card = person("Ada", "ada@x.org");
    card.add_phone("+49 30 1");
    card.add_organization("Engines");
    card.extra
        .insert("example.org:thing".into(), serde_json::json!(1));
    let key = w.add(card);
    let sheet = w.adapter.read(&key).unwrap();
    assert_eq!(sheet.title, "Ada");
    let all: Vec<(String, String)> = sheet
        .sections
        .iter()
        .flat_map(|s| s.fields.iter())
        .map(|f| (f.label.clone(), f.value.clone()))
        .collect();
    assert!(all.contains(&("E-mail".into(), "ada@x.org".into())));
    assert!(all.contains(&("Phone".into(), "+49 30 1".into())));
    assert!(all.contains(&("Organisation".into(), "Engines".into())));
    assert!(all.iter().any(|(_, v)| v.contains("example.org:thing")));
}

#[test]
fn a_new_person_is_stored_listed_and_selected() {
    let w = World::in_memory();
    let mut s = w.state();
    s.begin_create();
    s.set_field("name/full", "Grace");
    s.set_field("emails/+0", " grace@x.org ");
    s.set_field("phones/+0", "+1 202 555");
    s.set_field("organizations/+0", "Navy");
    s.save();
    assert!(
        s.editing().is_none(),
        "{:?}",
        s.editing().map(|e| e.error.clone())
    );
    let key = s.selected().unwrap().to_string();
    let card = w.card(&key);
    assert_eq!(card.display_name(), "Grace");
    assert_eq!(card.kind(), "individual");
    assert_eq!(card.emails["e1"].address, "grace@x.org", "trimmed");
    assert_eq!(card.phones["p1"].number, "+1 202 555");
    assert_eq!(card.organizations["o1"].name.as_deref(), Some("Navy"));
    assert_eq!(s.rows().len(), 1);
}

#[test]
fn a_new_person_with_blanks_only_is_refused_with_a_reason_and_stays_in_the_form() {
    let w = World::in_memory();
    let mut s = w.state();
    s.begin_create();
    s.set_field("name/full", "   ");
    s.save();
    let edit = s.editing().expect("the form stays");
    assert!(edit.error.as_deref().unwrap().contains("at least"));
    assert!(names(&w, "").is_empty());
}

#[test]
fn a_new_person_may_be_only_a_mail_address_and_is_called_by_it() {
    let w = World::in_memory();
    let mut s = w.state();
    s.begin_create();
    s.set_field("emails/+0", "only@x.org");
    s.save();
    assert!(
        s.editing().is_none(),
        "{:?}",
        s.editing().map(|e| e.error.clone())
    );
    assert_eq!(names(&w, ""), ["only@x.org"]);
}

#[test]
fn deleting_takes_two_presses_and_takes_the_roles_along() {
    let w = World::in_memory();
    let ada = w.add(person("Ada", "ada@x.org"));
    w.add(person("Bob", "bob@x.org"));
    let id = ada.parse().unwrap();
    w.runtime.block_on(w.repo.mark_ai(id)).unwrap();
    let mut s = w.state();
    assert_eq!(s.selected(), Some(ada.as_str()));
    s.ask_delete();
    assert_eq!(names(&w, "").len(), 2, "only asked");
    s.confirm_delete();
    assert_eq!(names(&w, ""), ["Bob"]);
    assert!(w.runtime.block_on(w.repo.roles_of(id)).unwrap().is_empty());
    assert!(matches!(s.shown(), Shown::Person { sheet, .. } if sheet.title == "Bob"));
}

#[test]
fn a_person_gone_or_a_key_that_names_nobody_is_not_found() {
    let w = World::in_memory();
    let key = w.add(person("Ada", "ada@x.org"));
    w.runtime
        .block_on(w.repo.delete(key.parse().unwrap()))
        .unwrap();
    assert_eq!(w.adapter.read(&key).unwrap_err().kind, ErrorKind::NotFound);
    assert_eq!(
        w.adapter.read("not an id").unwrap_err().kind,
        ErrorKind::NotFound
    );
}

#[test]
fn the_capabilities_of_the_provider_decide_what_is_offered() {
    let w = World::in_memory();
    let can = w.adapter.capabilities();
    assert!(can.edit && can.create && can.delete);
}

fn secret(name: &str) -> Card {
    let mut card = person(name, "s@x.org");
    card.mark_sensitive(["pd:HealthRecord"]);
    card
}

#[test]
fn someone_sensitive_is_marked_in_the_list_in_search_and_in_the_form_and_not_opened_alone() {
    let w = World::in_memory();
    let key = w.add(secret("Aaron Secret"));
    w.add(person("Zoe Open", "z@x.org"));
    let mut s = w.state();
    assert_eq!(s.rows()[0].cells[0], "Aaron Secret");
    assert_eq!(s.rows()[0].marks, ["Sensitive"]);
    assert!(s.rows()[1].marks.is_empty());
    assert_eq!(
        s.selected(),
        None,
        "first in the list, and not opened until chosen"
    );
    s.search("Aaron");
    assert_eq!(s.rows()[0].marks, ["Sensitive"], "in a search as well");
    s.select(&key);
    assert_eq!(s.selected(), Some(key.as_str()), "chosen, so shown");
    s.begin_edit();
    assert_eq!(
        s.editing().unwrap().draft.marks,
        ["Sensitive: pd:HealthRecord"],
        "the form keeps showing it"
    );
    let compact = fs_components_workbenches::people::PeopleState::new(
        w.adapter.clone(),
        fs_people_app::labels(),
        fs_components_workbenches::people::Variant::Compact,
    );
    assert_eq!(
        compact.rows()[0].marks,
        ["Sensitive"],
        "in the compact variant"
    );
    assert_eq!(compact.selected(), None);
}

#[test]
fn after_a_delete_someone_sensitive_is_not_opened_alone() {
    let w = World::in_memory();
    w.add(person("Ada Open", "a@x.org"));
    w.add(secret("Bob Secret"));
    let mut s = w.state();
    s.ask_delete();
    s.confirm_delete();
    assert_eq!(s.rows()[0].cells[0], "Bob Secret");
    assert_eq!(s.selected(), None);
}

#[test]
fn e_mail_and_phone_fields_are_checked_by_the_core_values_with_the_programs_words() {
    use fs_components_workbenches::people::FieldCheck;
    let w = World::in_memory();
    let key = w.add(person("Ada", "ada@x.org"));
    let draft = w.adapter.draft(&key).unwrap();
    let kinds: Vec<(String, FieldCheck)> =
        draft.fields().map(|f| (f.key.clone(), f.check)).collect();
    assert!(
        kinds.contains(&("emails/e1".into(), FieldCheck::Email)),
        "{kinds:?}"
    );
    assert!(
        w.adapter
            .check(FieldCheck::Email, "ada@bücher.example")
            .is_none()
    );
    assert!(
        w.adapter.check(FieldCheck::Email, "").is_none(),
        "empty removes the value"
    );
    let said = w.adapter.check(FieldCheck::Email, "ada").unwrap();
    assert!(said.contains('@'), "{said}");
    assert!(
        w.adapter
            .check(FieldCheck::Phone, "+49 30 1234567")
            .is_none()
    );
    assert!(
        w.adapter
            .check(FieldCheck::Phone, "030 1234567")
            .unwrap()
            .contains('+')
    );
    assert!(w.adapter.check(FieldCheck::Text, "anything").is_none());
    // A field added by the person holds the same kind.
    let blank = w.adapter.blank().unwrap();
    assert!(
        blank
            .groups
            .iter()
            .any(|g| g.key == "phones" && g.check == FieldCheck::Phone)
    );
}

#[test]
fn the_photo_of_a_card_is_a_data_uri_and_a_web_address_is_never_fetched() {
    let w = World::in_memory();
    let with = |uri: &str| -> Card {
        serde_json::from_value(serde_json::json!({
            "@type": "Card", "version": "1.0", "uid": ObjectId::new().to_string(),
            "name": {"full": "Ada"},
            "media": {"m1": {"@type": "Media", "kind": "photo", "uri": uri}},
        }))
        .unwrap()
    };
    let pixel = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    let key = w.add(with(pixel));
    assert_eq!(w.adapter.read(&key).unwrap().image.as_deref(), Some(pixel));
    let key = w.add(with("https://example.org/ada.png"));
    assert_eq!(
        w.adapter.read(&key).unwrap().image,
        None,
        "no call out when opening a person"
    );
    let key = w.add(person("Bob", "bob@x.org"));
    assert_eq!(w.adapter.read(&key).unwrap().image, None);
}
