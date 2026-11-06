//! JSContact JSON as an [`Importer`] / [`Exporter`].
//!
//! Pattern: Adapter — the native wire form of the model (one card or an array of cards).

use serde_json::Value;

use super::traits::{Exporter, ImportItems, ImportedCard, Importer};
use crate::card::Card;
use crate::error::Result;
use crate::validate::inexact_numbers;

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

    fn import_each<'a>(&'a self, text: &'a str) -> Result<ImportItems<'a>> {
        let items = match serde_json::from_str::<Value>(text)? {
            Value::Array(a) => a,
            other => vec![other],
        };
        // Read from the text: what the parser changed cannot be told afterwards.
        let mut inexact = inexact_numbers(text).into_iter().peekable();
        Ok(Box::new(items.into_iter().enumerate().map(
            move |(i, item)| {
                if let Some((_, token)) = inexact.next_if(|(at, _)| *at == i) {
                    return Err(crate::validate::inexact(&token));
                }
                let card = serde_json::from_value::<Card>(item)?;
                Ok(ImportedCard {
                    card,
                    original: None,
                })
            },
        )))
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
