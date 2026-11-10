//! `change`: an [`Edit`] (field path plus typed values, field by field) applied to a card.
//!
//! Pattern: Command over a Strategy table — every field of the description has one writer, which
//! turns the values of the form back into the card's sub-objects. The entries a form cannot show
//! (their `contexts`, `pref`, `label`, or an entry without the shown value) are kept with the values
//! that remain. The repository turns the difference into the card's PatchObject (`repo/commands.rs`).

use fs_types::{BasicRules, DescribesType, Edit};

use super::fields::{self, Field};
use super::sync::sync;
use crate::card::Card;
use crate::card::{
    Address, Anniversary, AnniversaryDate, Calendar, CryptoKey, Directory, EmailAddress,
    LanguagePref, Link, Media, Name, Nickname, Note, OnlineService, Organization, PersonalInfo,
    Phone, Pronouns, SchedulingAddress, SpeakToAs, Title,
};
use crate::error::{Error, Issue, Result};

fn refuse(path: String, code: &str) -> Error {
    Error::Invalid(vec![Issue {
        path,
        message: code.into(),
    }])
}

/// The card with `edit` applied. Nothing is changed in `card`.
pub fn change(card: &Card, edit: &Edit) -> Result<Card> {
    let rules = BasicRules::standard();
    let checked = edit.check(&Card::describe(), &rules).map_err(|issues| {
        Error::Invalid(
            issues
                .into_iter()
                .map(|i| Issue {
                    path: format!("/{}", i.field),
                    message: super::check::code_of(i.problem).to_owned(),
                })
                .collect(),
        )
    })?;
    let table = fields::table();
    let mut out = card.clone();
    for f in &checked.fields {
        let name = f.path[0].as_str();
        let field = table
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| refuse(format!("/{name}"), "unknown-field"))?;
        write(&mut out, field, &f.values, &rules)
            .map_err(|code| refuse(format!("/{name}"), code))?;
    }
    Ok(out)
}

fn one(values: &[String]) -> Option<String> {
    values.first().cloned()
}

fn parse_day(value: &str) -> Option<(u32, u32, u32)> {
    let mut parts = value.split('-').map(|p| p.parse::<u32>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

#[allow(clippy::too_many_lines)] // one arm per field of the table; splitting would hide the table
fn write(
    card: &mut Card,
    f: &Field,
    v: &[String],
    rules: &BasicRules,
) -> std::result::Result<(), &'static str> {
    macro_rules! list {
        ($map:expr, $p:literal, $read:expr, $make:expr) => {
            sync(&mut $map, $p, f, v, rules, $read, $make)
        };
    }
    match f.name {
        "uid" | "created" | "updated" | "prodId" => return Err("read-only"),
        "kind" => card.kind = one(v),
        "language" => card.language = one(v),
        "members" => {
            card.members = v.iter().map(|u| (u.clone(), true)).collect();
        }
        "relatedTo" => {
            card.related_to.retain(|k, _| v.contains(k));
            for u in v {
                card.related_to
                    .entry(u.clone())
                    .or_insert_with(|| serde_json::json!({"@type": "Relation"}));
            }
        }
        "name" => {
            // The name as the form shows it, sent back unchanged, leaves the name as it is (components, sortAs ...).
            let shown = fields::name_text(card);
            match one(v) {
                Some(full) if shown.as_deref() == Some(full.as_str()) => {}
                Some(full) => {
                    let name = card.name.get_or_insert_with(Name::default);
                    name.components.clear();
                    name.is_ordered = None;
                    name.default_separator = None;
                    name.full = Some(full);
                }
                None => card.name = None,
            }
        }
        "nicknames" => list!(card.nicknames, "n", |o| Some(o.name.clone()), |s| {
            Nickname {
                name: s.into(),
                ..Nickname::default()
            }
        }),
        "organizations" => list!(card.organizations, "o", |o| o.name.clone(), |s| {
            Organization {
                name: Some(s.into()),
                ..Organization::default()
            }
        }),
        "titles" => list!(card.titles, "t", |o| Some(o.name.clone()), |s| Title {
            name: s.into(),
            ..Title::default()
        }),
        "grammaticalGender" => {
            let speak = card.speak_as.get_or_insert_with(SpeakToAs::default);
            speak.grammatical_gender = one(v);
        }
        "pronouns" => {
            let speak = card.speak_as.get_or_insert_with(SpeakToAs::default);
            sync(
                &mut speak.pronouns,
                "p",
                f,
                v,
                rules,
                |o: &Pronouns| Some(o.pronouns.clone()),
                |s| Pronouns {
                    pronouns: s.into(),
                    ..Pronouns::default()
                },
            );
        }
        "emails" => list!(card.emails, "e", |o| Some(o.address.clone()), |s| {
            EmailAddress {
                address: s.into(),
                ..EmailAddress::default()
            }
        }),
        "onlineServices" => list!(card.online_services, "s", |o| o.uri.clone(), |s| {
            OnlineService {
                uri: Some(s.into()),
                ..OnlineService::default()
            }
        }),
        "phones" => list!(card.phones, "p", |o| Some(o.number.clone()), |s| Phone {
            number: s.into(),
            ..Phone::default()
        }),
        "preferredLanguages" => list!(
            card.preferred_languages,
            "l",
            |o| Some(o.language.clone()),
            |s| LanguagePref {
                language: s.into(),
                ..LanguagePref::default()
            }
        ),
        "calendars" => list!(card.calendars, "c", |o| Some(o.uri.clone()), |s| Calendar {
            kind: Some("calendar".into()),
            uri: s.into(),
            ..Calendar::default()
        }),
        "schedulingAddresses" => list!(
            card.scheduling_addresses,
            "a",
            |o| Some(o.uri.clone()),
            |s| SchedulingAddress {
                uri: s.into(),
                ..SchedulingAddress::default()
            }
        ),
        "addresses" => list!(card.addresses, "ad", fields::address_text, |s| Address {
            full: Some(s.into()),
            ..Address::default()
        }),
        "cryptoKeys" => list!(card.crypto_keys, "k", |o| fields::not_data(&o.uri), |s| {
            CryptoKey {
                uri: s.into(),
                ..CryptoKey::default()
            }
        }),
        "directories" => list!(card.directories, "d", |o| Some(o.uri.clone()), |s| {
            Directory {
                kind: Some("directory".into()),
                uri: s.into(),
                ..Directory::default()
            }
        }),
        "links" => list!(card.links, "w", |o| Some(o.uri.clone()), |s| Link {
            uri: s.into(),
            ..Link::default()
        }),
        "media" => list!(card.media, "m", |o| fields::not_data(&o.uri), |s| Media {
            kind: Some("photo".into()),
            uri: s.into(),
            ..Media::default()
        }),
        "anniversaries" => list!(card.anniversaries, "an", fields::day_of, |s| {
            let (year, month, day) =
                parse_day(s).map_or((None, None, None), |(y, m, d)| (Some(y), Some(m), Some(d)));
            Anniversary {
                date: Some(AnniversaryDate {
                    type_: Some("PartialDate".into()),
                    year,
                    month,
                    day,
                    ..AnniversaryDate::default()
                }),
                ..Anniversary::default()
            }
        }),
        "keywords" => {
            // Sent back as shown (the joined text), the set stays; a keyword that holds a comma cannot be told apart
            // from two, which is the limit of this one-text form.
            if fields::keywords_text(card) != one(v) {
                card.keywords = one(v)
                    .iter()
                    .flat_map(|k| k.split(','))
                    .map(|w| (w.trim().to_owned(), true))
                    .filter(|(w, _)| !w.is_empty())
                    .collect();
            }
        }
        "notes" => list!(card.notes, "n", |o| Some(o.note.clone()), |s| Note {
            note: s.into(),
            ..Note::default()
        }),
        "personalInfo" => list!(card.personal_info, "i", |o| Some(o.value.clone()), |s| {
            PersonalInfo {
                kind: Some("interest".into()),
                value: s.into(),
                ..PersonalInfo::default()
            }
        }),
        _ => return Err("unknown-field"),
    }
    if let Some(speak) = &card.speak_as
        && speak.grammatical_gender.is_none()
        && speak.pronouns.is_empty()
        && speak.extra.is_empty()
    {
        card.speak_as = None;
    }
    Ok(())
}
