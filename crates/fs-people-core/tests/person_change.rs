//! The command `change`: an `Edit` (typed values, field by field) against the trait `People`, on a real store.

mod common;

use common::{person, repo};
use fs_people_core::{Card, Error, People};
use fs_types::{BasicType, DescribesType, Edit};
use serde_json::json;

async fn stored(r: &fs_people_core::PeopleRepo, card: Card) -> fs_objects::ObjectId {
    r.create(card).await.unwrap().envelope.id
}

fn code(e: &Error) -> Vec<(String, String)> {
    match e {
        Error::Invalid(issues) => issues
            .iter()
            .map(|i| (i.path.clone(), i.message.clone()))
            .collect(),
        other => panic!("{other}"),
    }
}

#[tokio::test]
async fn the_values_replace_the_field_and_an_entry_that_stays_keeps_its_details() {
    let r = repo().await;
    let id = stored(&r, person("Ada", "ada@example.org")).await;
    // The details a form does not show.
    let rich: Card = {
        let mut c = r.get_card(id).await.unwrap().unwrap();
        c.emails.get_mut("e1").unwrap().label = Some("main".into());
        c
    };
    People::update(&r, id, rich).await.unwrap();
    let edit = Edit::new().set(
        &["emails"],
        BasicType::Email,
        &["ada@example.org", "second@example.com"],
    );
    let done = People::change(&r, id, &edit).await.unwrap();
    let card: Card = serde_json::from_value(done.content).unwrap();
    assert_eq!(card.emails.len(), 2);
    assert_eq!(
        card.emails["e1"].label.as_deref(),
        Some("main"),
        "the kept entry keeps its label"
    );
    assert_eq!(card.emails["e2"].address, "second@example.com");
    // Drop the first, by leaving it out.
    let edit = Edit::new().set(&["emails"], BasicType::Email, &["second@example.com"]);
    let card: Card =
        serde_json::from_value(People::change(&r, id, &edit).await.unwrap().content).unwrap();
    assert_eq!(card.emails.keys().collect::<Vec<_>>(), ["e2"]);
}

#[tokio::test]
async fn no_values_clear_the_field_and_a_field_not_named_stays() {
    let r = repo().await;
    let mut c = person("Ada", "ada@example.org");
    c.add_phone("+49 30 123456");
    let id = stored(&r, c).await;
    let edit = Edit::new().clear(&["phones"], BasicType::Phone);
    let card: Card =
        serde_json::from_value(People::change(&r, id, &edit).await.unwrap().content).unwrap();
    assert!(card.phones.is_empty());
    assert_eq!(card.emails.len(), 1);
    assert_eq!(card.name.unwrap().full.as_deref(), Some("Ada"));
}

#[tokio::test]
async fn entries_that_show_no_value_stay() {
    let r = repo().await;
    let mut c = person("Ada", "ada@example.org");
    c.online_services.insert(
        "s1".into(),
        serde_json::from_value(json!({"service": "Mastodon", "user": "ada"})).unwrap(),
    );
    let id = stored(&r, c).await;
    let edit = Edit::new().set(
        &["onlineServices"],
        BasicType::Link,
        &["https://social.example/@ada"],
    );
    let card: Card =
        serde_json::from_value(People::change(&r, id, &edit).await.unwrap().content).unwrap();
    assert_eq!(
        card.online_services.len(),
        2,
        "the entry with only a user is not the form's to drop"
    );
    assert_eq!(card.online_services["s1"].user.as_deref(), Some("ada"));
}

#[tokio::test]
async fn single_fields_name_kind_gender_keywords() {
    let r = repo().await;
    let id = stored(&r, person("Ada", "ada@example.org")).await;
    let edit = Edit::new()
        .set(&["name"], BasicType::Name, &["Ada King"])
        .set(&["grammaticalGender"], BasicType::Choice, &["feminine"])
        .set(&["keywords"], BasicType::Keywords, &["math, Engines"]);
    let card: Card =
        serde_json::from_value(People::change(&r, id, &edit).await.unwrap().content).unwrap();
    assert_eq!(
        card.name.as_ref().unwrap().full.as_deref(),
        Some("Ada King")
    );
    assert_eq!(
        card.speak_as
            .as_ref()
            .unwrap()
            .grammatical_gender
            .as_deref(),
        Some("feminine")
    );
    assert_eq!(
        card.keywords.keys().collect::<Vec<_>>(),
        ["engines", "math"]
    );
    let edit = Edit::new().clear(&["grammaticalGender"], BasicType::Choice);
    let card: Card =
        serde_json::from_value(People::change(&r, id, &edit).await.unwrap().content).unwrap();
    assert!(card.speak_as.is_none(), "an emptied speakAs goes");
}

#[tokio::test]
async fn what_is_wrong_is_a_path_and_a_code_and_nothing_is_written() {
    let r = repo().await;
    let id = stored(&r, person("Ada", "ada@example.org")).await;
    let before = r.get(id).await.unwrap().unwrap().content;
    for (edit, expected) in [
        (
            Edit::new().set(&["emails"], BasicType::Email, &["nonsense"]),
            "/emails",
        ),
        (Edit::new().set(&["nope"], BasicType::Text, &["x"]), "/nope"),
        (
            Edit::new().set(&["emails"], BasicType::Phone, &["+4930123456"]),
            "/emails",
        ),
        (
            Edit::new().set(&["emails", "e1", "address"], BasicType::Email, &["a@b.org"]),
            "/emails/e1/address",
        ),
        (
            Edit::new().set(&["uid"], BasicType::Text, &["other"]),
            "/uid",
        ),
        (
            Edit::new().set(&["name"], BasicType::Name, &["A", "B"]),
            "/name",
        ),
    ] {
        let err = People::change(&r, id, &edit).await.unwrap_err();
        let found = code(&err);
        assert_eq!(found[0].0, expected, "{found:?}");
        assert!(
            !found[0].1.contains(' '),
            "a code, not a sentence: {found:?}"
        );
    }
    assert_eq!(r.get(id).await.unwrap().unwrap().content, before);
    // The valid part of a bad edit is not written either.
    let mixed = Edit::new()
        .set(&["name"], BasicType::Name, &["Changed"])
        .set(&["emails"], BasicType::Email, &["nonsense"]);
    People::change(&r, id, &mixed).await.unwrap_err();
    assert_eq!(r.get(id).await.unwrap().unwrap().content, before);
}

#[tokio::test]
async fn an_edit_that_changes_nothing_writes_nothing() {
    let r = repo().await;
    let id = stored(&r, person("Ada", "ada@example.org")).await;
    let before = r.get(id).await.unwrap().unwrap();
    let edit = Edit::new().set(&["emails"], BasicType::Email, &["ada@example.org"]);
    let again = People::change(&r, id, &edit).await.unwrap();
    assert_eq!(again.content, before.content);
}

#[tokio::test]
async fn every_field_of_the_description_can_be_changed_or_is_named_read_only() {
    let r = repo().await;
    let sample = |b: BasicType, choices: &[String]| -> String {
        match b {
            BasicType::Email => "x@example.org".into(),
            BasicType::Phone => "+4930999999".into(),
            BasicType::Name | BasicType::Text | BasicType::MultilineText => "Sample".into(),
            BasicType::Address => "1 Main St, Berlin".into(),
            BasicType::Date => "2000-01-02".into(),
            BasicType::DateTime => "2026-01-02T03:04:05Z".into(),
            BasicType::Language => "en".into(),
            BasicType::Link => "https://example.org/x".into(),
            BasicType::Image => "https://example.org/x.png".into(),
            BasicType::Keywords => "alpha".into(),
            BasicType::Choice => choices[0].clone(),
            BasicType::Reference => "urn:uuid:11111111-1111-4111-8111-111111111111".into(),
            other => panic!("{other:?}"),
        }
    };
    for f in Card::describe().fields {
        let id = stored(&r, Card::group(format!("G {}", f.name))).await;
        let edit = Edit::new().set(
            &[f.name.as_str()],
            f.basic,
            &[sample(f.basic, &f.choices).as_str()],
        );
        match People::change(&r, id, &edit).await {
            Ok(done) => {
                let card: Card = serde_json::from_value(done.content).unwrap();
                let shown = fs_people_core::person::values_of(&card, &f.name).unwrap();
                let wanted = fs_types::BasicRules::standard()
                    .check(f.basic, &sample(f.basic, &f.choices), &f.choices)
                    .unwrap();
                assert!(
                    shown.contains(&wanted),
                    "{}: wrote {wanted:?}, the card shows {shown:?}",
                    f.name
                );
            }
            Err(e) => {
                let found = code(&e);
                assert!(
                    found.iter().all(|(_, c)| c == "read-only"),
                    "{}: {found:?}",
                    f.name
                );
                assert!(["uid", "created", "updated", "prodId"].contains(&f.name.as_str()));
            }
        }
    }
}

#[tokio::test]
async fn what_a_form_shows_sent_back_unchanged_changes_nothing() {
    let r = repo().await;
    let full: Card = serde_json::from_str(include_str!("fixtures/full_card.json")).unwrap();
    let id = stored(&r, full).await;
    let before = r.get_card(id).await.unwrap().unwrap();
    for f in Card::describe().fields {
        if ["uid", "created", "updated", "prodId"].contains(&f.name.as_str()) {
            continue;
        }
        let shown = fs_people_core::person::values_of(&before, &f.name).unwrap();
        if shown.is_empty() {
            continue;
        }
        let values: Vec<&str> = shown.iter().map(String::as_str).collect();
        let edit = Edit::new().set(&[f.name.as_str()], f.basic, &values);
        let done = People::change(&r, id, &edit)
            .await
            .unwrap_or_else(|e| panic!("{}: {e}", f.name));
        let after: Card = serde_json::from_value(done.content).unwrap();
        let (mut a, mut b) = (
            serde_json::to_value(&before).unwrap(),
            serde_json::to_value(&after).unwrap(),
        );
        for v in [&mut a, &mut b] {
            v.as_object_mut().unwrap().remove("updated");
        }
        assert_eq!(a, b, "{} came back different", f.name);
    }
}

#[tokio::test]
async fn a_new_value_never_takes_the_id_of_an_entry_still_there_or_just_gone() {
    let r = repo().await;
    let mut c = person("Ada", "a@example.org");
    c.add_email("b@example.org");
    let id = stored(&r, c).await;
    let edit = Edit::new().set(
        &["emails"],
        BasicType::Email,
        &["c@example.org", "a@example.org"],
    );
    let card: Card =
        serde_json::from_value(People::change(&r, id, &edit).await.unwrap().content).unwrap();
    let mut got: Vec<_> = card.emails.values().map(|e| e.address.as_str()).collect();
    got.sort_unstable();
    assert_eq!(got, ["a@example.org", "c@example.org"]);
    assert!(card.emails.contains_key("e1"), "a stays e1");
    assert!(
        !card.emails.contains_key("e2"),
        "the id of the dropped b is not reused"
    );
}

#[tokio::test]
async fn a_shown_value_that_is_older_than_the_rules_sent_back_is_not_a_change() {
    let r = repo().await;
    let card: Card = serde_json::from_value(json!({
        "@type": "Card", "version": "1.0", "uid": "urn:uuid:9f8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
        "kind": "example.com:robot",
        "keywords": {"Rust": true},
        "name": {"isOrdered": true, "sortAs": {"surname": "L"}, "components": [
            {"kind": "surname", "value": "Lovelace"}, {"kind": "separator", "value": ", "}, {"kind": "given", "value": "Ada"}]}
    }))
    .unwrap();
    let id = stored(&r, card).await;
    let before = r.get(id).await.unwrap().unwrap().content;
    let shown_name =
        fs_people_core::person::values_of(&r.get_card(id).await.unwrap().unwrap(), "name").unwrap();
    let edit = Edit::new()
        .set(&["kind"], BasicType::Choice, &["example.com:robot"])
        .set(&["keywords"], BasicType::Keywords, &["Rust"])
        .set(&["name"], BasicType::Name, &[shown_name[0].as_str()]);
    let done = People::change(&r, id, &edit).await.unwrap();
    assert_eq!(done.content, before);
}

#[tokio::test]
async fn a_path_named_twice_is_refused_even_when_one_of_them_is_the_shown_value() {
    let r = repo().await;
    let id = stored(&r, Card::individual("Ada")).await;
    let before = r.get(id).await.unwrap().unwrap().content;
    let edit = Edit::new()
        .set(&["kind"], BasicType::Choice, &["group"])
        .set(&["kind"], BasicType::Choice, &["individual"]);
    let err = People::change(&r, id, &edit).await.unwrap_err();
    assert_eq!(code(&err)[0].1, "duplicate-path");
    assert_eq!(r.get(id).await.unwrap().unwrap().content, before);
}
