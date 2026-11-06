//! A form written back onto a card.
//!
//! Pattern: Command — only the fields that differ from what the form started with are applied, each
//! to the entry its key names. Everything else on the card stays untouched: unknown properties,
//! contexts, preferences, labels, addresses, notes, name parts of a name that was not changed.
//!
//! Changes come first, additions second, removals last, so an id freed by this edit is never given
//! to something added by it. An emptied field removes that value, and a title no longer points at an
//! organisation that went. A changed name replaces the name's parts as well: left in place they
//! would keep the old name in sorting, search and vCard. An organisation that has units only loses
//! its name when emptied. The sensitive categories of the card never shrink (`marks.rs`).

use fs_components_workbenches::SourceError;
use fs_components_workbenches::people::{DraftField, PersonDraft};
use fs_people_core::Card;
use fs_people_core::card::{Name, SENSITIVE_KEY};

use super::draft::{EMAILS, NAME, ORGANISATIONS, PHONES};
use super::marks::Marks;

/// What a name may lose together with its text: it only describes that text.
const NAME_ONLY: [&str; 3] = ["sortAs", "phoneticScript", "phoneticSystem"];

/// Put what changed in `draft` onto `card`.
///
/// # Errors
/// [`SourceError`] for a field this form does not know (nothing may be dropped silently), for a name
/// that holds more than the form shows when it is emptied, and for a card that would be left with
/// nothing to call the person by. The rule is the same for a new card and a stored one.
pub fn apply_draft(card: &mut Card, draft: &PersonDraft) -> Result<(), SourceError> {
    let marks = Marks::of(card);
    let changed: Vec<&DraftField> = draft.fields().filter(|f| f.changed()).collect();
    let mut removals = Vec::new();
    let mut additions = Vec::new();
    for field in changed {
        let value = field.value.trim();
        if field.key == NAME {
            set_name(card, value)?;
            continue;
        }
        let Some((group, id)) = field.key.split_once('/') else {
            return Err(unknown(field));
        };
        if !matches!(group, EMAILS | PHONES | ORGANISATIONS) {
            return Err(unknown(field));
        }
        if value.is_empty() {
            removals.push((group, id));
        } else if !change(card, group, id, value) {
            additions.push((group, value));
        }
    }
    for (group, value) in additions {
        add(card, group, value);
    }
    for (group, id) in removals {
        remove(card, group, id);
    }
    marks.restore(card);
    if has_something_to_call_it(card) {
        Ok(())
    } else {
        Err(SourceError::failed(
            "enter at least a name, an e-mail, a phone or an organisation",
        ))
    }
}

fn unknown(field: &DraftField) -> SourceError {
    SourceError::failed(format!("the form has an unknown field \"{}\"", field.key))
}

fn has_something_to_call_it(card: &Card) -> bool {
    card.name.is_some()
        || !card.nicknames.is_empty()
        || !card.emails.is_empty()
        || !card.phones.is_empty()
        || !card.organizations.is_empty()
}

fn set_name(card: &mut Card, value: &str) -> Result<(), SourceError> {
    if value.is_empty() {
        let kept: Vec<&str> = card
            .name
            .iter()
            .flat_map(|n| n.extra.keys())
            .map(String::as_str)
            .filter(|key| !NAME_ONLY.contains(key) && *key != SENSITIVE_KEY)
            .collect();
        if !kept.is_empty() {
            return Err(SourceError::failed(format!(
                "this name holds more than the form shows ({}), so it can be replaced but not emptied",
                kept.join(", ")
            )));
        }
        card.name = None;
        return Ok(());
    }
    let name = card.name.get_or_insert_with(|| Name {
        type_: Some("Name".into()),
        ..Name::default()
    });
    name.components.clear();
    name.is_ordered = None;
    name.default_separator = None;
    name.full = Some(value.to_string());
    Ok(())
}

/// Change the entry `id`; `false` if there is none.
fn change(card: &mut Card, group: &str, id: &str, value: &str) -> bool {
    match group {
        EMAILS => card
            .emails
            .get_mut(id)
            .map(|e| value.clone_into(&mut e.address)),
        PHONES => card
            .phones
            .get_mut(id)
            .map(|p| value.clone_into(&mut p.number)),
        _ => card
            .organizations
            .get_mut(id)
            .map(|o| o.name = Some(value.to_string())),
    }
    .is_some()
}

fn add(card: &mut Card, group: &str, value: &str) {
    match group {
        EMAILS => card.add_email(value),
        PHONES => card.add_phone(value),
        _ => card.add_organization(value),
    };
}

fn remove(card: &mut Card, group: &str, id: &str) {
    match group {
        EMAILS => {
            card.emails.remove(id);
        }
        PHONES => {
            card.phones.remove(id);
        }
        _ => match card.organizations.get_mut(id) {
            Some(organisation) if !organisation.units.is_empty() => organisation.name = None,
            _ => {
                card.organizations.remove(id);
                for title in card.titles.values_mut() {
                    if title.organization_id.as_deref() == Some(id) {
                        title.organization_id = None;
                    }
                }
            }
        },
    }
}
