//! The description of `Person`: what the program tells the world about its data type (ADR 0066).
//! Tests against the trait `DescribesType` and the registry of `fs-types`, not against the table behind.

use fs_people_core::Card;
use fs_types::{BasicRules, BasicType, CommandKind, DescribesType, TypeRegistry};
use serde_json::Value;

const FULL: &str = include_str!("fixtures/full_card.json");

/// JSContact properties of the card that the description does not carry as a field of their own, and why.
const NOT_FIELDS: &[(&str, &str)] = &[
    ("@type", "discriminator, always Card"),
    ("version", "fixed by the standard"),
    (
        "speakAs",
        "its members are the fields grammaticalGender and pronouns",
    ),
    (
        "localizations",
        "no basic type for a PatchObject map yet: Unsupported",
    ),
    (
        "freesynergy.net:sensitive",
        "vendor property, not a person field",
    ),
];

#[test]
fn the_description_is_sound_and_registers() {
    let mut registry = TypeRegistry::new();
    registry.register::<Card>().expect("a sound description");
    let person = registry.get("Person").expect("registered under its name");
    assert_eq!(person.program, "fs-people");
    assert_eq!(person.standard, "RFC 9553 Card");
}

#[test]
fn every_property_of_the_card_is_a_field_or_named_as_not_one() {
    let person = Card::describe();
    let card: Value = serde_json::from_str(FULL).unwrap();
    for key in card.as_object().unwrap().keys() {
        let named = NOT_FIELDS.iter().any(|(k, _)| k == key);
        assert!(
            named || person.find_field(key).is_some(),
            "{key} is neither a field of Person nor listed as not one"
        );
    }
    for (key, _) in NOT_FIELDS {
        assert!(
            person.find_field(key).is_none() || *key == "speakAs",
            "{key} listed but described"
        );
    }
}

#[test]
fn the_fields_have_the_basic_types_of_the_standard() {
    let p = Card::describe();
    let basic = |n: &str| p.find_field(n).unwrap_or_else(|| panic!("{n}")).basic;
    assert_eq!(basic("emails"), BasicType::Email);
    assert_eq!(basic("phones"), BasicType::Phone);
    assert_eq!(basic("name"), BasicType::Name);
    assert_eq!(basic("addresses"), BasicType::Address);
    assert_eq!(basic("preferredLanguages"), BasicType::Language);
    assert_eq!(basic("links"), BasicType::Link);
    assert_eq!(basic("media"), BasicType::Image);
    assert_eq!(basic("anniversaries"), BasicType::Date);
    assert_eq!(basic("keywords"), BasicType::Keywords);
    assert_eq!(basic("members"), BasicType::Reference);
    assert_eq!(p.find_field("members").unwrap().target, "Person");
    assert!(p.find_field("emails").unwrap().multiple);
    assert!(!p.find_field("name").unwrap().multiple);
    assert!(
        p.find_field("kind")
            .unwrap()
            .choices
            .contains(&"group".to_owned())
    );
    assert!(p.fields.iter().all(|f| f.label.starts_with("person.")));
}

#[test]
fn the_commands_are_the_ones_the_task_names() {
    let p = Card::describe();
    let kinds: Vec<(&str, CommandKind)> = p
        .commands
        .iter()
        .map(|c| (c.name.as_str(), c.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("create", CommandKind::Create),
            ("change", CommandKind::Change),
            ("find", CommandKind::Find),
            ("delete", CommandKind::Delete),
            ("check", CommandKind::Check),
            ("suggest", CommandKind::Suggest),
        ]
    );
    // `change` takes an Edit and no fields of its own; no JSON text in a form (rules of types.capnp).
    let change = p.find_command("change").unwrap();
    assert!(change.input.is_empty() && change.supported);
    assert!(p.find_command("patch").is_none());
    // A list is the standard `find`: an empty term lists everybody, paging is the cursor of the call.
    let find = p.find_command("find").unwrap();
    assert!(find.input.iter().all(|f| !f.required));
    assert!(p.commands.iter().all(|c| c.supported));
}

#[test]
fn the_card_of_the_fixture_passes_the_check_of_its_own_description() {
    // The record a frontend would send, read from the card by the same table the check uses.
    let card: Card = serde_json::from_str(FULL).unwrap();
    let found = fs_people_core::person::check(&card);
    assert!(found.is_empty(), "{found:?}");
    let person = Card::describe();
    let rules = BasicRules::standard();
    assert_eq!(
        person
            .check_value(&rules, "emails", " Ada@Example.org ")
            .unwrap(),
        "Ada@example.org"
    );
}
