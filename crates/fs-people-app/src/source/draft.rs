//! A card as a form.
//!
//! Pattern: Adapter — the fields the form edits (name, e-mails, phones, organisations) as neutral
//! labelled fields. A field's key names what it belongs to on the card (`emails/e1`), so a save
//! finds the very entry again and keeps everything else about it.

use fs_components_workbenches::people::{DraftField, DraftGroup, PersonDraft};
use fs_people_core::Card;

use crate::words::field;

/// The key of the name field.
pub(super) const NAME: &str = "name/full";
/// The group of e-mail addresses.
pub(super) const EMAILS: &str = "emails";
/// The group of phone numbers.
pub(super) const PHONES: &str = "phones";
/// The group of organisations.
pub(super) const ORGANISATIONS: &str = "organizations";

/// The name as the form shows it: `name.full`, else the joined parts. Not the display name, which
/// falls back on an e-mail address.
pub(super) fn written_name(card: &Card) -> String {
    let Some(name) = &card.name else {
        return String::new();
    };
    if let Some(full) = name.full.as_deref().filter(|f| !f.trim().is_empty()) {
        return full.to_string();
    }
    let parts: Vec<&str> = name
        .components
        .iter()
        .filter(|c| c.kind != "separator" && !c.value.trim().is_empty())
        .map(|c| c.value.trim())
        .collect();
    parts.join(" ")
}

/// `card` as a form, marked with `revision`.
pub(super) fn draft_of(card: &Card, revision: Option<String>) -> PersonDraft {
    let entries = |group: &str, label: &str, values: Vec<(&String, String)>| {
        values
            .into_iter()
            .map(|(id, value)| DraftField::new(format!("{group}/{id}"), label, value))
            .collect::<Vec<_>>()
    };
    let emails = card
        .emails
        .iter()
        .map(|(id, e)| (id, e.address.clone()))
        .collect();
    let phones = card
        .phones
        .iter()
        .map(|(id, p)| (id, p.number.clone()))
        .collect();
    let organisations = card
        .organizations
        .iter()
        .map(|(id, o)| (id, o.name.clone().unwrap_or_default()))
        .collect();
    PersonDraft::new(
        revision,
        vec![
            DraftGroup::new(
                "name",
                field::NAME,
                false,
                vec![DraftField::new(NAME, field::NAME, written_name(card))],
            ),
            DraftGroup::new(
                EMAILS,
                field::EMAIL,
                true,
                entries(EMAILS, field::EMAIL, emails),
            ),
            DraftGroup::new(
                PHONES,
                field::PHONE,
                true,
                entries(PHONES, field::PHONE, phones),
            ),
            DraftGroup::new(
                ORGANISATIONS,
                field::ORGANISATION,
                true,
                entries(ORGANISATIONS, field::ORGANISATION, organisations),
            ),
        ],
    )
}

/// The form for a new person: one empty field of each kind.
pub(super) fn blank_draft() -> PersonDraft {
    let added = |group: &str, label: &str| {
        let mut one = DraftField::new(format!("{group}/+0"), label, "");
        one.added = true;
        DraftGroup::new(group, label, true, vec![one])
    };
    PersonDraft::new(
        None,
        vec![
            DraftGroup::new(
                "name",
                field::NAME,
                false,
                vec![DraftField::new(NAME, field::NAME, "")],
            ),
            added(EMAILS, field::EMAIL),
            added(PHONES, field::PHONE),
            added(ORGANISATIONS, field::ORGANISATION),
        ],
    )
}
