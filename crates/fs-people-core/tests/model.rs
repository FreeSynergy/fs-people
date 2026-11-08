//! Card model: lossless round trip and validation (RFC 9553).

mod common;

use common::{DEMO, person};
use fs_people_core::{Card, validate};
use serde_json::{Value, json};

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
    c.uid = String::new();
    c.version = "2.0".into();
    c.kind = Some("robot".into());
    c.emails.get_mut("e1").unwrap().address = String::new();
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
