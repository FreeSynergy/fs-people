//! JSContact JSON as an [`Importer`] / [`Exporter`].
//!
//! Pattern: Adapter — the native wire form of the model (one card or an array of cards).

use serde_json::Value;

use super::traits::{Exporter, ImportedCard, Importer};
use crate::card::Card;
use crate::error::Result;

/// JSContact JSON (RFC 9553): a single card or an array of cards.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsContactFormat;

impl Importer for JsContactFormat {
    fn media_type(&self) -> &'static str {
        "application/jscontact+json"
    }

    fn import(&self, text: &str) -> Result<Vec<Result<ImportedCard>>> {
        let items = match serde_json::from_str::<Value>(text)? {
            Value::Array(a) => a,
            other => vec![other],
        };
        Ok(items
            .into_iter()
            .map(|item| {
                let card = serde_json::from_value::<Card>(item)?;
                Ok(ImportedCard {
                    card,
                    original: None,
                })
            })
            .collect())
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
