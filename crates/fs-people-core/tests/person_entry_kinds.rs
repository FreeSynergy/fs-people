//! The kind of a new entry: asked as a choice that the description defines, preselected, and refused when it is not
//! one of the allowed kinds.

mod common;

use common::{person, repo};
use fs_people_core::{Card, Error, People};
use fs_types::{BasicType, DescribesType, Edit};

const ROWS: [(&str, &str, &str, BasicType, &str); 5] = [
    (
        "calendars",
        "calendarKind",
        "calendar",
        BasicType::Link,
        "https://cal.example/a",
    ),
    (
        "directories",
        "directoryKind",
        "directory",
        BasicType::Link,
        "ldap://ldap.example/o=X",
    ),
    (
        "media",
        "mediaKind",
        "photo",
        BasicType::Image,
        "https://example.org/a.png",
    ),
    (
        "personalInfo",
        "personalInfoKind",
        "interest",
        BasicType::Text,
        "chess",
    ),
    (
        "anniversaries",
        "anniversaryKind",
        "birth",
        BasicType::Date,
        "1815-12-10",
    ),
];

fn kind_of(card: &Card, field: &str) -> Vec<String> {
    let v = serde_json::to_value(card).unwrap();
    v[field]
        .as_object()
        .unwrap()
        .values()
        .map(|e| e["kind"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn the_description_reports_the_allowed_kinds_and_the_first_is_the_default() {
    let p = Card::describe();
    for (_, selector, default, ..) in ROWS {
        let f = p
            .find_field(selector)
            .unwrap_or_else(|| panic!("{selector}"));
        assert_eq!(f.basic, BasicType::Choice);
        assert!(!f.multiple && !f.read_only);
        assert_eq!(
            f.choices[0], default,
            "the_first_kind_is_the_default: {selector}"
        );
    }
    let media = p.find_field("mediaKind").unwrap();
    assert_eq!(
        media.choices,
        ["photo", "logo"],
        "an image field takes no sound"
    );
    assert_eq!(
        p.find_field("calendarKind").unwrap().choices,
        ["calendar", "free-busy"]
    );
}

#[tokio::test]
async fn a_new_entry_gets_the_chosen_kind_and_else_the_preselected_one() {
    let r = repo().await;
    for (field, selector, default, basic, value) in ROWS {
        let id = r
            .create(person("Ada", "ada@example.org"))
            .await
            .unwrap()
            .envelope
            .id;
        let plain = Edit::new().set(&[field], basic, &[value]);
        let card: Card =
            serde_json::from_value(People::change(&r, id, &plain).await.unwrap().content).unwrap();
        assert_eq!(kind_of(&card, field), [default], "{field} without a choice");

        let other = ROWS_OTHER.iter().find(|(f, _)| *f == field).unwrap().1;
        let id = r
            .create(person("Bob", "bob@example.org"))
            .await
            .unwrap()
            .envelope
            .id;
        let chosen = Edit::new()
            .set(&[selector], BasicType::Choice, &[other])
            .set(&[field], basic, &[value]);
        let card: Card =
            serde_json::from_value(People::change(&r, id, &chosen).await.unwrap().content).unwrap();
        let rfc = if other == "free-busy" {
            "freeBusy"
        } else {
            other
        };
        assert_eq!(kind_of(&card, field), [rfc], "{field} with {other}");
    }
}

const ROWS_OTHER: [(&str, &str); 5] = [
    ("calendars", "free-busy"),
    ("directories", "entry"),
    ("media", "logo"),
    ("personalInfo", "hobby"),
    ("anniversaries", "death"),
];

#[tokio::test]
async fn an_entry_that_exists_keeps_its_kind() {
    let r = repo().await;
    let id = r
        .create(person("Ada", "ada@example.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let first = Edit::new()
        .set(&["mediaKind"], BasicType::Choice, &["logo"])
        .set(&["media"], BasicType::Image, &["https://example.org/a.png"]);
    People::change(&r, id, &first).await.unwrap();
    // A second image, with the default kind: the first stays a logo.
    let second = Edit::new().set(
        &["media"],
        BasicType::Image,
        &["https://example.org/a.png", "https://example.org/b.png"],
    );
    let card: Card =
        serde_json::from_value(People::change(&r, id, &second).await.unwrap().content).unwrap();
    let mut kinds = kind_of(&card, "media");
    kinds.sort();
    assert_eq!(kinds, ["logo", "photo"]);
}

#[tokio::test]
async fn a_kind_that_is_not_allowed_is_refused_and_nothing_is_written() {
    let r = repo().await;
    let id = r
        .create(person("Ada", "ada@example.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let before = r.get(id).await.unwrap().unwrap().content;
    for (selector, bad) in [
        ("mediaKind", "video"),
        ("mediaKind", "sound"),
        ("calendarKind", "freeBusy"),
        ("personalInfoKind", "Interest"),
    ] {
        let edit = Edit::new().set(&[selector], BasicType::Choice, &[bad]).set(
            &["links"],
            BasicType::Link,
            &["https://example.org/x"],
        );
        let Err(Error::Invalid(issues)) = People::change(&r, id, &edit).await else {
            panic!("{selector} {bad} was taken");
        };
        assert_eq!(issues[0].path, format!("/{selector}"));
        assert_eq!(issues[0].message, "choice.not-in-list");
    }
    assert_eq!(r.get(id).await.unwrap().unwrap().content, before);
}

#[tokio::test]
async fn a_kind_alone_changes_nothing() {
    let r = repo().await;
    let id = r
        .create(person("Ada", "ada@example.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let before = r.get(id).await.unwrap().unwrap().content;
    let edit = Edit::new().set(&["mediaKind"], BasicType::Choice, &["logo"]);
    assert_eq!(People::change(&r, id, &edit).await.unwrap().content, before);
}

#[tokio::test]
async fn a_sound_on_the_card_is_not_the_image_forms_to_show_or_drop() {
    let r = repo().await;
    let mut c = person("Ada", "ada@example.org");
    c.media.insert(
        "m1".into(),
        serde_json::from_value(
            serde_json::json!({"kind": "sound", "uri": "https://example.org/hello.ogg"}),
        )
        .unwrap(),
    );
    let id = r.create(c).await.unwrap().envelope.id;
    let card = r.get_card(id).await.unwrap().unwrap();
    assert!(
        fs_people_core::person::values_of(&card, "media")
            .unwrap()
            .is_empty(),
        "a sound is no image"
    );
    let edit = Edit::new().set(&["media"], BasicType::Image, &["https://example.org/a.png"]);
    let card: Card =
        serde_json::from_value(People::change(&r, id, &edit).await.unwrap().content).unwrap();
    assert_eq!(card.media.len(), 2, "the sound stays");
    assert_eq!(card.media["m1"].kind.as_deref(), Some("sound"));
}
