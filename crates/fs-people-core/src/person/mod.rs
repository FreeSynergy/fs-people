//! What a person is as a data type: its description, patch, check and suggestions.
//!
//! Pattern: Facade — declares the concept files; the description is `impl DescribesType for Card`.

mod check;
mod describe;
mod edit;
mod fields;
mod patch;
mod suggest;
mod sync;

pub use check::check;

/// The values of a field of the description as a form shows them (empty when the card has none, `None` for a name that is
/// no field).
#[must_use]
pub fn values_of(card: &crate::card::Card, field: &str) -> Option<Vec<String>> {
    fields::table()
        .iter()
        .find(|f| f.name == field)
        .map(|f| (f.read)(card).into_iter().map(|(_, v)| v).collect())
}
pub use describe::{PERSON, PROGRAM};
pub use edit::change;
pub use patch::PatchObject;
pub use suggest::{MAX_SUGGESTIONS, SCAN_LIMIT, is_suggestible, suggest};
