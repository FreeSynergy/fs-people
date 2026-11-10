//! Every property of the RFC 9553 `Card` (and of each sub-object) is a typed member: nothing of the
//! standard sits in an `extra` map, and the card comes back as it went in. The fixture holds one
//! value for every property of the table in `docs/en/data/people/jscontact/description.md`.

use fs_people_core::Card;
use fs_people_core::card::Extra;
use serde_json::{Value, json};

const FULL: &str = include_str!("fixtures/full_card.json");

fn parsed() -> Card {
    serde_json::from_str(FULL).expect("the full card is a Card")
}

#[test]
fn the_full_card_comes_back_as_it_went_in() {
    let card = parsed();
    let again = serde_json::to_value(&card).unwrap();
    let original: Value = serde_json::from_str(FULL).unwrap();
    assert_eq!(again, original);
}

#[test]
fn nothing_of_the_standard_is_left_in_an_extra_map() {
    let card = parsed();
    let only_vendor: Vec<&String> = card.extra.keys().collect();
    assert_eq!(only_vendor, ["freesynergy.net:sensitive"]);
    let nested: Vec<(&str, &Extra)> = vec![
        ("name", &card.name.as_ref().unwrap().extra),
        (
            "name component",
            &card.name.as_ref().unwrap().components[1].extra,
        ),
        ("nickname", &card.nicknames["n1"].extra),
        ("organization", &card.organizations["o1"].extra),
        ("unit", &card.organizations["o1"].units[0].extra),
        ("speakAs", &card.speak_as.as_ref().unwrap().extra),
        (
            "pronouns",
            &card.speak_as.as_ref().unwrap().pronouns["p1"].extra,
        ),
        ("title", &card.titles["t1"].extra),
        ("email", &card.emails["e1"].extra),
        ("onlineService", &card.online_services["s1"].extra),
        ("phone", &card.phones["p1"].extra),
        ("language", &card.preferred_languages["l1"].extra),
        ("calendar", &card.calendars["c1"].extra),
        ("scheduling", &card.scheduling_addresses["a1"].extra),
        ("address", &card.addresses["ad1"].extra),
        (
            "address component",
            &card.addresses["ad1"].components[1].extra,
        ),
        ("cryptoKey", &card.crypto_keys["k1"].extra),
        ("directory", &card.directories["d1"].extra),
        ("link", &card.links["w1"].extra),
        ("media", &card.media["m1"].extra),
        ("birth", &card.anniversaries["b"].extra),
        (
            "birth date",
            &card.anniversaries["b"].date.as_ref().unwrap().extra,
        ),
        (
            "death date",
            &card.anniversaries["d"].date.as_ref().unwrap().extra,
        ),
        ("note", &card.notes["n1"].extra),
        ("author", &card.notes["n1"].author.as_ref().unwrap().extra),
        ("personalInfo", &card.personal_info["i1"].extra),
    ];
    for (what, extra) in nested {
        assert!(extra.is_empty(), "{what} keeps {:?} untyped", extra.keys());
    }
}

#[test]
fn the_typed_members_hold_the_values() {
    let card = parsed();
    assert_eq!(card.language.as_deref(), Some("en-GB"));
    assert_eq!(
        card.prod_id.as_deref(),
        Some("-//example.org//people 1//EN")
    );
    let name = card.name.as_ref().unwrap();
    assert_eq!(name.sort_as.as_deref(), Some("Lovelace, Ada"));
    assert_eq!(name.phonetic_system.as_deref(), Some("ipa"));
    assert_eq!(name.components[1].phonetic.as_deref(), Some("AY-duh"));
    assert_eq!(
        card.speak_as
            .as_ref()
            .unwrap()
            .grammatical_gender
            .as_deref(),
        Some("feminine")
    );
    assert_eq!(
        card.online_services["s1"].service.as_deref(),
        Some("Mastodon")
    );
    assert_eq!(card.preferred_languages["l1"].language, "en");
    assert_eq!(card.calendars["c1"].uri, "https://cal.example/ada");
    assert_eq!(
        card.scheduling_addresses["a1"].uri,
        "mailto:ada@example.org"
    );
    assert_eq!(card.addresses["ad1"].pref, Some(1));
    assert_eq!(card.addresses["ad1"].is_ordered, Some(true));
    assert_eq!(
        card.crypto_keys["k1"].media_type.as_deref(),
        Some("application/pgp-keys")
    );
    assert_eq!(card.directories["d1"].list_as, Some(3));
    assert_eq!(card.links["w1"].media_type.as_deref(), Some("text/html"));
    assert_eq!(card.media["m1"].blob_id.as_deref(), Some("B1"));
    assert_eq!(
        card.localizations["de"]["titles/t1/name"],
        json!("Mathematikerin")
    );
    let birth = card.anniversaries["b"].date.as_ref().unwrap();
    assert_eq!(
        (birth.year, birth.month, birth.day),
        (Some(1815), Some(12), Some(10))
    );
    assert_eq!(
        card.anniversaries["d"]
            .date
            .as_ref()
            .unwrap()
            .utc
            .as_deref(),
        Some("1852-11-27T00:00:00Z")
    );
    assert_eq!(
        card.notes["n1"].author.as_ref().unwrap().name.as_deref(),
        Some("Charles")
    );
    assert_eq!(card.personal_info["i1"].level.as_deref(), Some("high"));
}

#[test]
fn the_full_card_is_valid() {
    fs_people_core::validate::validate(&parsed()).unwrap();
}

#[test]
fn a_card_of_an_older_version_keeps_every_property() {
    // Stored before these members were typed: they sat in `extra` and must come back unchanged.
    let older = json!({
        "@type": "Card", "version": "1.0", "uid": "u", "name": {"full": "A"},
        "onlineServices": {"s": {"service": "X", "someday:new": 1}},
        "personalInfo": {"i": {"kind": "hobby", "value": "chess"}},
        "someday:card": {"a": [1, 2]}
    });
    let card: Card = serde_json::from_value(older.clone()).unwrap();
    assert_eq!(serde_json::to_value(&card).unwrap(), older);
}
