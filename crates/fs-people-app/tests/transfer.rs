//! vCard files in and out, and a store refusing a sensitive card.
//!
//! Integration tests over the real core; files in a scratch folder.

mod common;

use common::{Scratch, World, person};
use fs_components_workbenches::people::PeopleSource;
use fs_people_app::{export_all, import_file};
use fs_people_core::{Card, vcard::card_to_vcard};

const TWO: &str = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:urn:uuid:01a1aaef-ba80-7f7a-8dff-9f85769beae1\r\nFN:Ada\r\nEMAIL:ada@x.org\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nFN:Bob\r\nTEL:+49 30 1\r\nEND:VCARD\r\n";

fn sensitive_vcard() -> String {
    let mut card = person("Secret", "s@x.org");
    card.mark_sensitive(["pd:SexualPreference"]);
    card_to_vcard(&card).unwrap()
}

#[test]
fn a_file_is_imported_and_known_people_are_skipped() {
    let (w, dir) = (World::in_memory(), Scratch::new());
    let file = dir.file("two.vcf");
    std::fs::write(&file, TWO).unwrap();
    let first = import_file(w.people.as_ref(), w.runtime.handle(), &file).unwrap();
    assert_eq!((first.created, first.skipped), (2, 0));
    assert!(first.is_clean(), "{}", first.message());
    let again = import_file(w.people.as_ref(), w.runtime.handle(), &file).unwrap();
    assert_eq!((again.created, again.skipped), (0, 2));
    assert_eq!(w.adapter.rows("", 0, 10).unwrap().len(), 2);
}

#[test]
fn a_missing_file_and_a_file_without_a_vcard_are_errors() {
    let (w, dir) = (World::in_memory(), Scratch::new());
    let missing = import_file(w.people.as_ref(), w.runtime.handle(), &dir.file("none.vcf"));
    assert!(missing.unwrap_err().contains("cannot read"));
    let file = dir.file("text.vcf");
    std::fs::write(&file, "just text").unwrap();
    let empty = import_file(w.people.as_ref(), w.runtime.handle(), &file);
    assert!(empty.unwrap_err().contains("no vCard"));
}

#[test]
fn a_card_that_cannot_be_read_is_reported_and_the_others_arrive() {
    let (w, dir) = (World::in_memory(), Scratch::new());
    let file = dir.file("mixed.vcf");
    let broken = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Ok\r\nKIND:not valid kind\r\nEND:VCARD\r\n";
    std::fs::write(&file, format!("{TWO}{broken}")).unwrap();
    let summary = import_file(w.people.as_ref(), w.runtime.handle(), &file).unwrap();
    assert_eq!(
        summary.created + summary.failed.len(),
        3,
        "{}",
        summary.message()
    );
    assert!(summary.created >= 2);
}

#[test]
fn a_plain_store_refusing_a_sensitive_card_is_an_error_in_plain_words_and_earlier_people_stay() {
    let dir = Scratch::new();
    let w = World::in_file(&dir.file("people.db"));
    let file = dir.file("mixed.vcf");
    std::fs::write(&file, format!("{TWO}{}", sensitive_vcard())).unwrap();
    let summary = import_file(w.people.as_ref(), w.runtime.handle(), &file).unwrap();
    assert!(!summary.is_clean());
    let message = summary.message();
    assert!(message.contains("sensitive"), "{message}");
    assert!(message.contains("not encrypted"), "{message}");
    assert_eq!(w.adapter.rows("", 0, 10).unwrap().len(), 2);
}

#[test]
fn everyone_is_exported_and_comes_back_elsewhere_and_a_taken_file_is_not_replaced() {
    let (w, dir) = (World::in_memory(), Scratch::new());
    let mut ada = person("Ada", "ada@x.org");
    ada.add_phone("+49 30 1");
    w.add(ada);
    w.add(Card::individual("Bob"));
    let file = dir.file("out.vcf");
    let report = export_all(w.people.as_ref(), w.runtime.handle(), &file, &[]).unwrap();
    assert_eq!(report.written, 2);
    let other = World::in_memory();
    let back = import_file(other.people.as_ref(), other.runtime.handle(), &file).unwrap();
    assert_eq!(back.created, 2, "{}", back.message());
    let rows = other.adapter.rows("", 0, 10).unwrap();
    assert_eq!(rows[0].cells, ["Ada", "ada@x.org"]);

    let before = std::fs::read_to_string(&file).unwrap();
    let again = export_all(w.people.as_ref(), w.runtime.handle(), &file, &[]);
    assert!(again.unwrap_err().contains("cannot create"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
}

#[test]
fn people_marked_sensitive_are_left_out_of_an_export_and_named() {
    let (w, dir) = (World::in_memory(), Scratch::new());
    let mut secret = person("Secret", "s@x.org");
    secret.mark_sensitive(["pd:SexualPreference"]);
    w.add(secret);
    w.add(person("Open", "o@x.org"));
    let file = dir.file("out.vcf");
    let report = export_all(w.people.as_ref(), w.runtime.handle(), &file, &[]).unwrap();
    assert_eq!(report.written, 1);
    assert!(report.left_out[0].starts_with("urn:uuid:"), "{report:?}");
    let said = report.message(&file);
    assert!(
        said.contains(&report.left_out[0]) && !said.contains("Secret"),
        "{said}"
    );
    assert!(!std::fs::read_to_string(&file).unwrap().contains("s@x.org"));
}

#[test]
fn an_export_after_an_edit_says_the_new_name_not_the_imported_one() {
    let (w, dir) = (World::in_memory(), Scratch::new());
    let source = dir.file("in.vcf");
    let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Ada Lovelace\r\nN:Lovelace;Ada;;;\r\nEMAIL:ada@x.org\r\nEND:VCARD\r\n";
    std::fs::write(&source, vcard).unwrap();
    import_file(w.people.as_ref(), w.runtime.handle(), &source).unwrap();
    let mut s = w.state();
    s.begin_edit();
    s.set_field("name/full", "Augusta King");
    let mail = s
        .editing()
        .unwrap()
        .draft
        .fields()
        .find(|f| f.key.starts_with("emails/"))
        .unwrap()
        .key
        .clone();
    s.set_field(&mail, "augusta@x.org");
    s.save();
    assert!(
        s.editing().is_none(),
        "{:?}",
        s.editing().map(|e| e.error.clone())
    );
    let out = dir.file("out.vcf");
    export_all(w.people.as_ref(), w.runtime.handle(), &out, &[]).unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("Augusta King"), "{text}");
    assert!(text.contains("augusta@x.org"), "{text}");
    assert!(!text.contains("Lovelace"), "the old name lives on: {text}");
    assert!(
        !text.contains("ada@x.org"),
        "the old address lives on: {text}"
    );
}
