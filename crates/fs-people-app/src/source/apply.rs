//! A form written back onto a card.
//!
//! Pattern: Command — only the fields that differ from what the form started with are applied, each
//! to the entry its key names. Everything else on the card stays untouched: unknown properties,
//! contexts, preferences, labels, addresses, notes, name parts of a name that was not changed.
//!
//! An emptied field removes that value. A changed name replaces the name's parts as well: left in
//! place they would keep the old name in sorting, search and vCard. An organisation that has units
//! only loses its name when emptied.

use fs_components_workbenches::SourceError;
use fs_components_workbenches::people::{DraftField, PersonDraft};
use fs_people_core::Card;
use fs_people_core::card::Name;

use super::draft::{EMAILS, NAME, ORGANISATIONS, PHONES};

/// Put what changed in `draft` onto `card`.
///
/// # Errors
/// [`SourceError`] for a field this form does not know: nothing may be dropped silently.
pub fn apply_draft(card: &mut Card, draft: &PersonDraft) -> Result<(), SourceError> {
    for field in draft.fields().filter(|f| f.changed()) {
        let value = field.value.trim();
        if field.key == NAME {
            set_name(card, value);
            continue;
        }
        let Some((group, id)) = field.key.split_once('/') else {
            return Err(unknown(field));
        };
        match group {
            EMAILS => set_email(card, id, value),
            PHONES => set_phone(card, id, value),
            ORGANISATIONS => set_organisation(card, id, value),
            _ => return Err(unknown(field)),
        }
    }
    Ok(())
}

fn unknown(field: &DraftField) -> SourceError {
    SourceError::failed(format!("the form has an unknown field \"{}\"", field.key))
}

fn set_name(card: &mut Card, value: &str) {
    let name = card.name.get_or_insert_with(|| Name {
        type_: Some("Name".into()),
        ..Name::default()
    });
    name.components.clear();
    name.is_ordered = None;
    name.default_separator = None;
    name.full = (!value.is_empty()).then(|| value.to_string());
    // Empty and with nothing else on it: no name, which a card may be without.
    if name.full.is_none() && name.extra.is_empty() {
        card.name = None;
    }
}

fn set_email(card: &mut Card, id: &str, value: &str) {
    match (card.emails.get_mut(id), value.is_empty()) {
        (Some(_), true) => {
            card.emails.remove(id);
        }
        (Some(entry), false) => value.clone_into(&mut entry.address),
        (None, true) => {}
        (None, false) => {
            card.add_email(value);
        }
    }
}

fn set_phone(card: &mut Card, id: &str, value: &str) {
    match (card.phones.get_mut(id), value.is_empty()) {
        (Some(_), true) => {
            card.phones.remove(id);
        }
        (Some(entry), false) => value.clone_into(&mut entry.number),
        (None, true) => {}
        (None, false) => {
            card.add_phone(value);
        }
    }
}

fn set_organisation(card: &mut Card, id: &str, value: &str) {
    match (card.organizations.get_mut(id), value.is_empty()) {
        (Some(entry), true) if !entry.units.is_empty() => entry.name = None,
        (Some(_), true) => {
            card.organizations.remove(id);
        }
        (Some(entry), false) => entry.name = Some(value.to_string()),
        (None, true) => {}
        (None, false) => {
            card.add_organization(value);
        }
    }
}
