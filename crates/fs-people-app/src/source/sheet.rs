//! A card as neutral sections of labelled fields, to be read.
//!
//! Pattern: Adapter — the JSContact card and its roles in the workbench's terms. Marks (an AI,
//! sensitive data) stand out at the top (ADR 0029, 0054). What the card holds that is not shown is
//! named, so nobody thinks it is gone.

use fs_components_workbenches::people::{PersonField, PersonRow, PersonSection, PersonSheet};
use fs_people_core::{Card, Error, RoleBook, card_of, is_ai_role};
use fs_objects::StoredObject;
use tokio::runtime::Handle;

use super::marks::is_marked;
use crate::words::field;

/// The row of a person. A card that cannot be parsed is still listed under its name, so it can be
/// found.
pub(super) fn row_of(object: &StoredObject) -> PersonRow {
    let mail = card_of(object)
        .ok()
        .as_ref()
        .map_or_else(String::new, first_email);
    let row = PersonRow::new(
        object.envelope.id.to_string(),
        vec![object.envelope.name.clone(), mail],
    );
    if is_marked(object) {
        row.guarded(field::SENSITIVE_ROW)
    } else {
        row
    }
}

/// The e-mail to show in a row: the preferred one, else the first.
pub(super) fn first_email(card: &Card) -> String {
    card.emails
        .values()
        .min_by_key(|e| e.pref.unwrap_or(u32::MAX))
        .map(|e| e.address.clone())
        .unwrap_or_default()
}

/// The sheet of `card`, stored as `object`.
pub(super) fn sheet_of(
    card: &Card,
    object: &StoredObject,
    roles: &dyn RoleBook,
    runtime: &Handle,
) -> PersonSheet {
    let (role_section, is_ai) = role_section(object, roles, runtime);
    let sections = [
        contact(card),
        work(card),
        places(card),
        about(card),
        role_section,
        kept(card),
    ];
    let mut sheet = PersonSheet::new(
        card.display_name(),
        sections
            .into_iter()
            .filter(|s| !s.fields.is_empty())
            .collect(),
    );
    if is_ai {
        sheet.marks.push(field::AI.to_string());
    }
    sheet.marks.extend(sensitive_mark(card, object));
    sheet
}

/// The marking of someone sensitive, with the categories; `None` for everyone else (ADR 0029).
pub(super) fn sensitive_mark(card: &Card, object: &StoredObject) -> Option<String> {
    is_marked(object).then(|| {
        let categories = card.sensitive_categories().join(", ");
        let categories = if categories.is_empty() {
            field::UNCLEAR
        } else {
            categories.as_str()
        };
        field::SENSITIVE.replacen("{}", categories, 1)
    })
}

fn contact(card: &Card) -> PersonSection {
    let mails = card
        .emails
        .values()
        .map(|e| PersonField::new(field::EMAIL, &e.address));
    let phones = card
        .phones
        .values()
        .map(|p| PersonField::new(field::PHONE, &p.number));
    let links = card
        .links
        .values()
        .map(|l| PersonField::new(field::LINK, &l.uri));
    PersonSection::new(field::CONTACT, mails.chain(phones).chain(links).collect())
}

fn work(card: &Card) -> PersonSection {
    let organisations = card
        .organizations
        .values()
        .filter_map(|o| o.name.as_ref())
        .map(|n| PersonField::new(field::ORGANISATION, n));
    let titles = card
        .titles
        .values()
        .map(|t| PersonField::new(field::JOB, &t.name));
    PersonSection::new(field::WORK, organisations.chain(titles).collect())
}

fn places(card: &Card) -> PersonSection {
    let lines = card.addresses.values().map(|a| {
        let joined = a
            .components
            .iter()
            .filter(|c| c.kind != "separator")
            .map(|c| c.value.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        PersonField::new(field::ADDRESS, a.full.clone().unwrap_or(joined))
    });
    PersonSection::new(field::PLACES, lines.collect())
}

fn about(card: &Card) -> PersonSection {
    let kind = (card.kind() != "individual").then(|| PersonField::new(field::KIND, card.kind()));
    let nicknames = card
        .nicknames
        .values()
        .map(|n| PersonField::new(field::NICKNAME, &n.name));
    let notes = card
        .notes
        .values()
        .map(|n| PersonField::new(field::NOTE, &n.note));
    PersonSection::new(
        field::ABOUT,
        kind.into_iter().chain(nicknames).chain(notes).collect(),
    )
}

/// The roles, read only, and whether one of them marks an AI. A provider without roles has no such
/// section; a failure is shown, not hidden.
fn role_section(
    object: &StoredObject,
    roles: &dyn RoleBook,
    runtime: &Handle,
) -> (PersonSection, bool) {
    let held = match runtime.block_on(roles.roles_of(object.envelope.id)) {
        Ok(held) => held,
        Err(Error::Unsupported(_)) => Vec::new(),
        Err(e) => {
            let why = field::UNREADABLE.replacen("{}", &e.to_string(), 1);
            let fields = vec![PersonField::new(field::ROLE, why)];
            return (PersonSection::new(field::ROLES, fields), false);
        }
    };
    let is_ai = held.iter().any(|a| is_ai_role(&a.role));
    let fields = held
        .iter()
        .map(|assignment| {
            let name = runtime
                .block_on(roles.role(&assignment.role))
                .ok()
                .flatten()
                .and_then(|d| {
                    d.names
                        .get("en")
                        .or_else(|| d.names.values().next())
                        .cloned()
                })
                .unwrap_or_else(|| assignment.role.clone());
            PersonField::new(field::ROLE, name)
        })
        .collect();
    (PersonSection::new(field::ROLES, fields), is_ai)
}

/// The names of what the card holds beyond what is shown above.
fn kept(card: &Card) -> PersonSection {
    let mut names: Vec<&str> = card.extra.keys().map(String::as_str).collect();
    if !card.members.is_empty() {
        names.push("members");
    }
    if !card.related_to.is_empty() {
        names.push("relatedTo");
    }
    if !card.keywords.is_empty() {
        names.push("keywords");
    }
    let fields = if names.is_empty() {
        Vec::new()
    } else {
        vec![PersonField::new(field::OTHER, names.join(", "))]
    };
    PersonSection::new(field::KEPT, fields)
}
