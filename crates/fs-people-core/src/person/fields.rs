//! The fields of a person as the type description names them: one table, read by both the
//! description and the record check.
//!
//! Pattern: Strategy table — each row is a JSContact property with its basic type and the function
//! that reads its values off a card. The description (`describe.rs`) and the check (`check.rs`) walk
//! the same rows, so what a frontend is told and what the program checks cannot drift apart.

use fs_types::BasicType;

use crate::card::{Card, Map};
use crate::validate::KINDS;

/// One value of a field with the id it has in the card (`e1`); an id-less property uses `""`.
pub(crate) type Read = (String, String);

/// One row of the table.
pub(crate) struct Field {
    /// The JSContact property (a sub-object member for `grammaticalGender` and `pronouns`: `speakAs`).
    pub name: &'static str,
    pub basic: BasicType,
    pub multiple: bool,
    /// The data type a `reference` points to.
    pub target: &'static str,
    /// The allowed keys of a `choice`.
    pub choices: &'static [&'static str],
    /// Whether `check` judges the values by the basic type. Off where the card rules already say it
    /// (`kind` may be a vendor value) or the value is no free input (`uid`).
    pub checked: bool,
    pub read: fn(&Card) -> Vec<Read>,
}

const ONE: &str = "";

fn one(value: Option<&String>) -> Vec<Read> {
    value
        .map(|v| (ONE.to_owned(), v.clone()))
        .into_iter()
        .collect()
}

fn each<T>(items: &Map<T>, pick: impl Fn(&T) -> Option<String>) -> Vec<Read> {
    items
        .iter()
        .filter_map(|(id, o)| pick(o).map(|v| (id.clone(), v)))
        .collect()
}

/// A URI that is worth judging as a link: a `data:` URI holds a file (a photo, a key), whose size
/// the limit of a link (2048 bytes) was not made for.
pub(crate) fn not_data(uri: &str) -> Option<String> {
    (!uri.starts_with("data:")).then(|| uri.to_owned())
}

fn name_of(card: &Card) -> Vec<Read> {
    let Some(name) = &card.name else {
        return vec![];
    };
    let text = name.full.clone().unwrap_or_else(|| {
        let sep = name.default_separator.as_deref().unwrap_or(" ");
        let parts: Vec<&str> = name.components.iter().map(|c| c.value.as_str()).collect();
        parts.join(sep)
    });
    vec![(ONE.to_owned(), text)]
}

pub(crate) fn address_text(a: &crate::card::Address) -> Option<String> {
    a.full.clone().or_else(|| {
        let sep = a.default_separator.as_deref().unwrap_or(" ");
        let parts: Vec<&str> = a.components.iter().map(|c| c.value.as_str()).collect();
        (!parts.is_empty()).then(|| parts.join(sep))
    })
}

/// Only a complete day counts; a year alone or a timestamp is not a `date`.
pub(crate) fn day_of(a: &crate::card::Anniversary) -> Option<String> {
    let d = a.date.as_ref()?;
    let scale_ok = d.calendar_scale.as_deref().is_none_or(|s| s == "gregorian");
    match (d.year, d.month, d.day) {
        (Some(y), Some(m), Some(day)) if scale_ok => Some(format!("{y:04}-{m:02}-{day:02}")),
        _ => None,
    }
}

fn keywords_of(card: &Card) -> Vec<Read> {
    if card.keywords.is_empty() {
        return vec![];
    }
    let words: Vec<&str> = card.keywords.keys().map(String::as_str).collect();
    vec![(ONE.to_owned(), words.join(", "))]
}

macro_rules! field {
    ($name:literal, $basic:ident, $multiple:expr, $read:expr) => {
        field!($name, $basic, $multiple, "", &[], true, $read)
    };
    ($name:literal, $basic:ident, $multiple:expr, $target:literal, $choices:expr, $checked:expr, $read:expr) => {
        Field {
            name: $name,
            basic: BasicType::$basic,
            multiple: $multiple,
            target: $target,
            choices: $choices,
            checked: $checked,
            read: $read,
        }
    };
}

/// RFC 9553 section 2.2.2.
pub(crate) const GENDERS: &[&str] = &[
    "animate",
    "common",
    "feminine",
    "inanimate",
    "masculine",
    "neuter",
];

/// All fields, in the order of the RFC sections.
pub(crate) fn table() -> Vec<Field> {
    vec![
        field!("uid", Text, false, "", &[], false, |c| one(Some(&c.uid))),
        field!("kind", Choice, false, "", &KINDS, false, |c| one(c
            .kind
            .as_ref())),
        field!("language", Language, false, |c| one(c.language.as_ref())),
        field!("prodId", Text, false, |c| one(c.prod_id.as_ref())),
        field!("created", DateTime, false, |c| one(c.created.as_ref())),
        field!("updated", DateTime, false, |c| one(c.updated.as_ref())),
        field!("members", Reference, true, "Person", &[], false, |c| {
            c.members.keys().map(|u| (u.clone(), u.clone())).collect()
        }),
        field!("relatedTo", Reference, true, "Person", &[], false, |c| {
            c.related_to
                .keys()
                .map(|u| (u.clone(), u.clone()))
                .collect()
        }),
        field!("name", Name, false, name_of),
        field!("nicknames", Text, true, |c| each(&c.nicknames, |o| Some(
            o.name.clone()
        ))),
        field!("organizations", Text, true, |c| each(
            &c.organizations,
            |o| o.name.clone()
        )),
        field!("titles", Text, true, |c| each(&c.titles, |o| Some(
            o.name.clone()
        ))),
        field!("grammaticalGender", Choice, false, "", GENDERS, true, |c| {
            one(c
                .speak_as
                .as_ref()
                .and_then(|s| s.grammatical_gender.as_ref()))
        }),
        field!("pronouns", Text, true, |c| {
            c.speak_as.as_ref().map_or_else(Vec::new, |s| {
                each(&s.pronouns, |o| Some(o.pronouns.clone()))
            })
        }),
        field!("emails", Email, true, |c| each(&c.emails, |o| Some(
            o.address.clone()
        ))),
        field!("onlineServices", Link, true, |c| each(
            &c.online_services,
            |o| o.uri.clone()
        )),
        field!("phones", Phone, true, |c| each(&c.phones, |o| Some(
            o.number.clone()
        ))),
        field!("preferredLanguages", Language, true, |c| {
            each(&c.preferred_languages, |o| Some(o.language.clone()))
        }),
        field!("calendars", Link, true, |c| each(&c.calendars, |o| Some(
            o.uri.clone()
        ))),
        field!("schedulingAddresses", Link, true, |c| {
            each(&c.scheduling_addresses, |o| Some(o.uri.clone()))
        }),
        field!("addresses", Address, true, |c| each(
            &c.addresses,
            address_text
        )),
        field!("cryptoKeys", Link, true, |c| each(&c.crypto_keys, |o| {
            not_data(&o.uri)
        })),
        field!("directories", Link, true, |c| each(&c.directories, |o| {
            Some(o.uri.clone())
        })),
        field!("links", Link, true, |c| each(&c.links, |o| Some(
            o.uri.clone()
        ))),
        field!("media", Image, true, |c| each(&c.media, |o| not_data(
            &o.uri
        ))),
        field!("anniversaries", Date, true, |c| each(
            &c.anniversaries,
            day_of
        )),
        field!("keywords", Keywords, false, keywords_of),
        field!("notes", MultilineText, true, |c| each(&c.notes, |o| Some(
            o.note.clone()
        ))),
        field!("personalInfo", Text, true, |c| each(
            &c.personal_info,
            |o| Some(o.value.clone())
        )),
    ]
}
