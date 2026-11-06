//! The sensitive marking of a card, which an edit never reduces, and how to tell it is there.
//!
//! Pattern: Memento — what the card was marked with is remembered before an edit and put back at
//! card level if the edit removed the part that carried it (ADR 0029). Looking for the marking in
//! the stored text fails closed: anything that mentions it counts as marked.

use std::collections::BTreeSet;

use fs_people_core::Card;
use fs_people_core::card::SENSITIVE_KEY;
use fs_store::StoredObject;
use serde_json::Value;

/// Stands in when a marking named no category and the part that carried it went.
const UNCLEAR: &str = "freesynergy.net:unclear";

/// Is this stored person marked sensitive in any form?
pub(crate) fn is_marked(object: &StoredObject) -> bool {
    !object.envelope.sensitive.is_empty() || object.content.to_string().contains(SENSITIVE_KEY)
}

/// What a card is marked with now.
pub(super) struct Marks {
    categories: BTreeSet<String>,
    marked: bool,
}

fn mentions(card: &Card) -> bool {
    card.to_value().to_string().contains(SENSITIVE_KEY)
}

impl Marks {
    pub(super) fn of(card: &Card) -> Self {
        Self {
            categories: card.sensitive_categories().into_iter().collect(),
            marked: mentions(card),
        }
    }

    /// Put back at card level whatever `card` has lost since.
    pub(super) fn restore(&self, card: &mut Card) {
        let present: BTreeSet<String> = card.sensitive_categories().into_iter().collect();
        let mut gone: Vec<String> = self.categories.difference(&present).cloned().collect();
        if gone.is_empty() && (!self.marked || mentions(card)) {
            return;
        }
        if gone.is_empty() {
            gone.push(UNCLEAR.to_string());
        }
        let mut level: Vec<String> = match card.extra.get(SENSITIVE_KEY) {
            Some(Value::Array(list)) => list
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            Some(Value::String(one)) => vec![one.clone()],
            _ => Vec::new(),
        };
        level.extend(gone);
        card.mark_sensitive(level);
    }
}
