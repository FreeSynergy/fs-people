//! Forms of older cards that are rewritten when the card is next saved.
//!
//! Pattern: Adapter (read old, write new) — reading accepts what was stored, writing only ever
//! produces the RFC form, so a store drifts to the standard one save at a time and nothing is lost.

use super::model::Card;
use super::parts::SortAs;

impl Card {
    /// Rewrite the older forms to RFC 9553: `name.sortAs` as one string becomes the map by component
    /// kind. Provisional (decision of the strategy, 2026-11-10): a string sorted the whole name, and the
    /// map's closest key for that is `surname`, the one a "surname, given" sort starts with. Returns
    /// whether anything changed.
    pub fn rewrite_legacy(&mut self) -> bool {
        let Some(name) = self.name.as_mut() else {
            return false;
        };
        let Some(SortAs::Whole(text)) = &name.sort_as else {
            return false;
        };
        name.sort_as = Some(SortAs::ByKind(
            [("surname".to_owned(), text.clone())].into(),
        ));
        true
    }
}
