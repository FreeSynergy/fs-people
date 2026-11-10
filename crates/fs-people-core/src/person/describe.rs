//! The description of a person: what a generated frontend is told (ADR 0066).
//!
//! Pattern: Registry entry (trait first) — [`Card`] implements `DescribesType` from `fs-types`, so
//! `TypeRegistry::register::<Card>()` puts it in the catalog the program serves. Fields come from the
//! table in `fields.rs`; the commands name what the program can do with a person.

use fs_types::{
    BasicType, CommandDescription, CommandKind, DescribesType, FieldDescription, TypeDescription,
};

use super::fields::{self, Field};
use crate::card::Card;

/// The name of the type in the catalog.
pub const PERSON: &str = "Person";

/// The program that owns it (ADR 0050).
pub const PROGRAM: &str = "fs-people";

/// `emailDomain` -> `email-domain`: the translation key part of a property name.
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn describe_field(f: &Field) -> FieldDescription {
    let mut d = FieldDescription::new(f.name, f.basic).label(&format!("person.{}", kebab(f.name)));
    if f.multiple {
        d = d.multiple();
    }
    if !f.target.is_empty() {
        d = d.target(f.target);
    }
    if !f.choices.is_empty() {
        d = d.choices(f.choices);
    }
    d
}

/// A field of a command: `person.<command>.<name>`.
fn input(command: &str, name: &str, basic: BasicType) -> FieldDescription {
    FieldDescription::new(name, basic).label(&format!("person.{command}.{}", kebab(name)))
}

fn person_ref(command: &str, name: &str) -> FieldDescription {
    input(command, name, BasicType::Reference).target(PERSON)
}

/// A person is described by its fields, which are the properties of the RFC 9553 card.
///
/// Not described (no basic type for it yet, `Unsupported`, see `DESIGN.md`): `localizations`, and
/// the members of a sub-object that are no basic type (`contexts`, `pref`, `label`, `features`,
/// `mediaType`).
impl DescribesType for Card {
    fn describe() -> TypeDescription {
        let mut t = TypeDescription::new(PERSON, PROGRAM, "RFC 9553 Card", "person");
        for f in &fields::table() {
            t = t.field(describe_field(f));
        }
        let kinds: Vec<&str> = crate::validate::KINDS.to_vec();
        t.command(
            CommandDescription::new("create", CommandKind::Create, "person.create")
                .input(input("create", "name", BasicType::Name))
                .input(input("create", "kind", BasicType::Choice).choices(&kinds))
                .input(input("create", "emails", BasicType::Email).multiple())
                .input(input("create", "phones", BasicType::Phone).multiple())
                .input(input("create", "organizations", BasicType::Text).multiple())
                .input(input("create", "keywords", BasicType::Keywords))
                .result(person_ref("create", "id").required()),
        )
        .command(
            CommandDescription::new("patch", CommandKind::Change, "person.patch")
                .input(person_ref("patch", "id").required())
                // A PatchObject (RFC 8984 section 1.4.9) has no basic type; it travels as JSON text.
                .input(input("patch", "patch", BasicType::MultilineText).required())
                .result(person_ref("patch", "id").required()),
        )
        .command(
            CommandDescription::new("find", CommandKind::Find, "person.find")
                .input(input("find", "term", BasicType::Text).required())
                .input(input("find", "kind", BasicType::Choice).choices(&kinds))
                .result(person_ref("find", "persons").multiple()),
        )
        .command(
            CommandDescription::new("delete", CommandKind::Delete, "person.delete")
                .input(person_ref("delete", "id").required()),
        )
        .command(
            CommandDescription::new("check", CommandKind::Check, "person.check")
                .input(person_ref("check", "id").required())
                .result(input("check", "problems", BasicType::Text).multiple()),
        )
        .command(
            CommandDescription::new("suggest", CommandKind::Suggest, "person.suggest")
                // The machine name of the field being typed in (`emails`); choice keys cannot hold camel case.
                .input(input("suggest", "field", BasicType::Text).required())
                .input(input("suggest", "typed", BasicType::Text).required())
                .result(input("suggest", "suggestions", BasicType::Text).multiple()),
        )
    }
}
