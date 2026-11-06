//! The [`Importer`] and [`Exporter`] contracts.
//!
//! Pattern: Strategy + Adapter — the repository depends on these traits only; every foreign
//! format (vCard now, CardDAV / CSV later) is an adapter implementing them.

use fs_store::Blob;

use crate::card::Card;
use crate::error::Result;

/// A card read from a foreign format, with the untouched source text when there is one.
#[derive(Clone, Debug)]
pub struct ImportedCard {
    /// The converted, validated card.
    pub card: Card,
    /// The source text of this card (ADR 0030), if the format has a text form worth keeping.
    pub original: Option<Blob>,
}

/// Reads cards from the text of one foreign format.
pub trait Importer: Send + Sync {
    /// Media type this importer reads (e.g. `text/vcard`).
    fn media_type(&self) -> &'static str;

    /// Parse `text` into cards. One entry per source item; items that cannot be converted are
    /// `Err` and reported by the caller, never silently dropped. `Err` for the whole call means
    /// the text is unusable as a whole.
    fn import(&self, text: &str) -> Result<Vec<Result<ImportedCard>>>;

    /// [`Importer::import`], one item at a time, so a large file does not stand in memory as cards.
    fn import_each<'a>(&'a self, text: &'a str) -> Result<ImportItems<'a>> {
        Ok(Box::new(self.import(text)?.into_iter()))
    }
}

/// The items of a file, as they are read.
pub type ImportItems<'a> = Box<dyn Iterator<Item = Result<ImportedCard>> + Send + 'a>;

/// Writes one card as text of one foreign format.
pub trait Exporter: Send + Sync {
    /// Media type this exporter produces.
    fn output_media_type(&self) -> &'static str;

    /// The card as text of this format.
    fn export(&self, card: &Card) -> Result<String>;
}
