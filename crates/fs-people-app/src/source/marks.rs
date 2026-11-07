//! The sensitive marking of a card, which an edit never reduces, and how to tell it is there.
//!
//! Pattern: Memento + Visitor — every marking on the card is found by walking its properties, the
//! same way the core does on `people-core-fixes` (main's core only knows the categories; replace
//! this walk with `Card::is_sensitive` when that branch is merged). A mere mention in a text is not
//! a marking. What an edit removed is put back at card level, and a marking that names no category
//! stays just that: it is moved as it came, never turned into a category.

use std::collections::BTreeSet;

use fs_people_core::Card;
use fs_people_core::card::SENSITIVE_KEY;
use fs_objects::StoredObject;
use serde_json::Value;

/// Is this stored person marked sensitive in any form?
pub(crate) fn is_marked(object: &StoredObject) -> bool {
    !object.envelope.sensitive.is_empty() || !markings(&object.content).is_empty()
}

/// The value of every marking in `value`.
fn markings(value: &Value) -> Vec<Value> {
    let mut found = Vec::new();
    collect(value, &mut found);
    found
}

fn collect(value: &Value, found: &mut Vec<Value>) {
    match value {
        Value::Object(members) => {
            for (key, member) in members {
                if key == SENSITIVE_KEY {
                    found.push(member.clone());
                } else {
                    collect(member, found);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect(item, found)),
        _ => {}
    }
}

/// Does this marking name categories, and nothing else?
fn names_categories(marking: &Value) -> bool {
    let named = |v: &Value| v.as_str().is_some_and(|t| !t.trim().is_empty());
    match marking {
        Value::Array(list) => !list.is_empty() && list.iter().all(named),
        other => named(other),
    }
}

/// What a card is marked with now.
pub(super) struct Marks {
    categories: BTreeSet<String>,
    unclear: Vec<Value>,
}

fn unclear_of(card: &Card) -> Vec<Value> {
    markings(&card.to_value())
        .into_iter()
        .filter(|marking| !names_categories(marking))
        .collect()
}

impl Marks {
    pub(super) fn of(card: &Card) -> Self {
        Self {
            categories: card.sensitive_categories().into_iter().collect(),
            unclear: unclear_of(card),
        }
    }

    /// Put back at card level whatever `card` has lost since.
    pub(super) fn restore(&self, card: &mut Card) {
        let present: BTreeSet<String> = card.sensitive_categories().into_iter().collect();
        let gone: Vec<Value> = self
            .categories
            .difference(&present)
            .cloned()
            .map(Value::String)
            .collect();
        let mut left = unclear_of(card);
        let mut gone_unclear = Vec::new();
        for marking in &self.unclear {
            match left.iter().position(|other| other == marking) {
                Some(found) => {
                    left.remove(found);
                }
                None => gone_unclear.push(marking.clone()),
            }
        }
        let level = card.extra.get(SENSITIVE_KEY).cloned();
        let value = match (level, gone.is_empty(), gone_unclear.as_slice()) {
            (_, true, []) => return,
            // Alone, it stays exactly what it was.
            (None, true, [one]) => one.clone(),
            (level, ..) => {
                let mut items = match level {
                    Some(Value::Array(items)) => items,
                    Some(other) => vec![other],
                    None => Vec::new(),
                };
                items.extend(gone);
                // Not a list of names any more, so it still reads as unclear.
                items.extend(gone_unclear);
                Value::Array(items)
            }
        };
        card.extra.insert(SENSITIVE_KEY.to_string(), value);
    }
}
