//! The values of a field as a form shows them.
//!
//! Pattern: Strategy table reader — the same rows `describe`, `check` and `edit` use.

use super::fields;
use crate::card::Card;

/// The values of a field of the description as a form shows them (empty when the card has none, `None` for a name that is
/// no field).
#[must_use]
pub fn values_of(card: &Card, field: &str) -> Option<Vec<String>> {
    fields::table()
        .iter()
        .find(|f| f.name == field)
        .map(|f| (f.read)(card).into_iter().map(|(_, v)| v).collect())
}
