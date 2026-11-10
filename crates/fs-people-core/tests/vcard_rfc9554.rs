//! vCard with the RFC 9554 parameters against the whole of RFC 9553: a card with every property goes out
//! as vCard 4.0 and comes back through `calcard`. What does not come back is named here, so that a
//! better `calcard` shows up as a failing test instead of an unnoticed change (`Unsupported`, DESIGN.md).

use fs_people_core::Card;
use fs_people_core::vcard::{card_from_vcard, card_to_vcard};
use serde_json::Value;

const FULL: &str = include_str!("fixtures/full_card.json");

/// What `calcard` 0.3.14 does not bring back, by JSON path.
const NOT_BACK: &[&str] = &[
    // ORG-DIRECTORY: calcard writes `label;uri` as the value and reads it back as the uri; the
    // parameters (CONTEXTS, MEDIATYPE) do not come back.
    "/directories/d1/uri",
    "/directories/d1/contexts",
    "/directories/d1/label",
    "/directories/d1/mediaType",
    // The calendar scale moves from the date to the anniversary (BDAY;CALSCALE).
    "/anniversaries/b/date/calendarScale",
];

/// Paths present in `a` and different or missing in `b`; `@type` of nested objects does not count
/// (RFC 9553 section 1.4.5: implied by the place, so a vCard does not carry it).
fn lost(a: &Value, b: &Value, at: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                if k == "@type" && !at.is_empty() {
                    continue;
                }
                match y.get(k) {
                    Some(w) => lost(v, w, &format!("{at}/{k}"), out),
                    None => out.push(format!("{at}/{k}")),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (v, w)) in x.iter().zip(y).enumerate() {
                lost(v, w, &format!("{at}/{i}"), out);
            }
        }
        _ if a != b => out.push(at.to_owned()),
        _ => {}
    }
}

fn card() -> Card {
    serde_json::from_str(FULL).unwrap()
}

#[test]
fn a_card_with_every_property_goes_out_as_vcard_and_back_with_the_named_exceptions() {
    let card = card();
    let text = card_to_vcard(&card).unwrap();
    assert!(text.starts_with("BEGIN:VCARD\r\nVERSION:4.0"), "{text}");
    let back = card_from_vcard(&text).unwrap();
    let mut found = Vec::new();
    lost(
        &serde_json::to_value(&card).unwrap(),
        &serde_json::to_value(&back).unwrap(),
        "",
        &mut found,
    );
    found.sort();
    let mut expected: Vec<String> = NOT_BACK.iter().map(|s| (*s).to_owned()).collect();
    expected.sort();
    assert_eq!(found, expected);
}

#[test]
fn the_rfc_9554_parameters_carry_what_vcard_has_no_property_for() {
    let text = card_to_vcard(&card()).unwrap();
    // PROP-ID keeps the ids of the sub-objects, JSCOMPS the order of address and name parts, JSPROP what has no
    // vCard form at all (the speakAs object).
    for needle in ["PROP-ID=e1", "JSCOMPS=", "JSPROP;JSPTR=speakAs"] {
        assert!(text.contains(needle), "{needle} missing in\n{text}");
    }
}

#[test]
fn the_ids_of_the_sub_objects_survive_the_round_trip() {
    let back = card_from_vcard(&card_to_vcard(&card()).unwrap()).unwrap();
    assert!(back.emails.contains_key("e1") && back.phones.contains_key("p1"));
    assert!(back.addresses.contains_key("ad1") && back.media.contains_key("m1"));
    assert_eq!(back.uid, card().uid);
}

#[test]
fn a_vcard_without_a_property_we_know_still_comes_in_whole() {
    let vcf = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Grace\r\nX-FUTURE;FOO=1:bar\r\nEND:VCARD\r\n";
    let card = card_from_vcard(vcf).unwrap();
    // What calcard cannot place stays on the card (extension `vCard`), so it can go out again.
    let out = card_to_vcard(&card).unwrap();
    assert!(out.contains("X-FUTURE"), "{out}");
}
