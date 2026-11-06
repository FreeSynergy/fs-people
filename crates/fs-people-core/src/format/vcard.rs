//! vCard as an [`Importer`] / [`Exporter`].
//!
//! Pattern: Adapter — [`VCardFormat`] puts the `calcard` based `vcard` module behind our traits.

use super::traits::{Exporter, ImportedCard, Importer};
use crate::card::Card;
use crate::error::Result;
use crate::vcard::{VCARD_MEDIA_TYPE, card_to_vcard, import_vcards};

/// vCard 2.1 / 3.0 / 4.0 in, vCard 4.0 out.
#[derive(Clone, Copy, Debug, Default)]
pub struct VCardFormat;

impl Importer for VCardFormat {
    fn media_type(&self) -> &'static str {
        VCARD_MEDIA_TYPE
    }

    fn import(&self, text: &str) -> Result<Vec<Result<ImportedCard>>> {
        Ok(import_vcards(text))
    }
}

impl Exporter for VCardFormat {
    fn output_media_type(&self) -> &'static str {
        VCARD_MEDIA_TYPE
    }

    fn export(&self, card: &Card) -> Result<String> {
        card_to_vcard(card)
    }
}
