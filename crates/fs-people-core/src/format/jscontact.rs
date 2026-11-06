//! JSContact JSON as an [`Importer`] / [`Exporter`].
//!
//! Pattern: Adapter — the native wire form of the model (one card or an array of cards).

use super::items::array_items;
use super::traits::{Exporter, ImportItems, ImportedCard, Importer};
use crate::card::Card;
use crate::error::Result;
use crate::validate::check_exact_numbers;

/// JSContact JSON (RFC 9553): a single card or an array of cards.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsContactFormat;

impl Importer for JsContactFormat {
    fn media_type(&self) -> &'static str {
        "application/jscontact+json"
    }

    fn import(&self, text: &str) -> Result<Vec<Result<ImportedCard>>> {
        Ok(self.import_each(text)?.collect())
    }

    /// Item by item from the text: one that no parser can read stays one failure, and what the parser
    /// would change is still there to be told.
    fn import_each<'a>(&'a self, text: &'a str) -> Result<ImportItems<'a>> {
        let items = if let Some(items) = array_items(text) {
            items
        } else {
            serde_json::from_str::<serde::de::IgnoredAny>(text)?;
            vec![text]
        };
        Ok(Box::new(items.into_iter().map(|item| {
            check_exact_numbers(item)?;
            let card = serde_json::from_str::<Card>(item)?;
            Ok(ImportedCard {
                card,
                original: None,
            })
        })))
    }
}

impl Exporter for JsContactFormat {
    fn output_media_type(&self) -> &'static str {
        "application/jscontact+json"
    }

    fn export(&self, card: &Card) -> Result<String> {
        Ok(serde_json::to_string_pretty(&card.to_value())?)
    }
}
