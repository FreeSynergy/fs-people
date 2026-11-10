//! `suggest`: completions for a value being typed, from what the program already holds.
//!
//! Pattern: Strategy by kind — the basic type's own rule completes (an e-mail address by the domains of
//! known addresses, a language, a link scheme); this file only finds the known values of a field
//! and hands them on. The cards read are bounded, so a large book is not scanned whole per key.

use fs_types::BasicRules;

use super::fields;
use crate::card::Card;
use crate::error::{Error, Issue, Result};

/// At most this many cards are read to learn the known values of a field.
pub const SCAN_LIMIT: usize = 1000;

/// Whether `field` is one the program completes.
#[must_use]
pub fn is_suggestible(field: &str) -> bool {
    fields::table().iter().any(|f| f.name == field && f.checked)
}

/// Completions of `typed` for `field`, given the cards to learn the known values from.
pub fn suggest<'a>(
    field: &str,
    typed: &str,
    cards: impl IntoIterator<Item = &'a Card>,
) -> Result<Vec<String>> {
    let table = fields::table();
    let Some(f) = table.iter().find(|f| f.name == field && f.checked) else {
        return Err(Error::Invalid(vec![Issue {
            path: "/field".into(),
            message: "unknown-field".into(),
        }]));
    };
    let rules = BasicRules::standard();
    let known: Vec<String> = cards
        .into_iter()
        .flat_map(|c| (f.read)(c))
        .filter_map(|(_, v)| rules.check(f.basic, &v, &[]).ok())
        .collect();
    let known: Vec<&str> = known.iter().map(String::as_str).collect();
    let choices: Vec<String> = f.choices.iter().map(|s| (*s).to_owned()).collect();
    Ok(rules.suggest(f.basic, typed, &known, &choices))
}
