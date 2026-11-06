//! vCard 4.0 (RFC 6350) via `calcard`: module map.
//!
//! Pattern: Adapter (declarations) — the files wrap the foreign `calcard` crate; the
//! `format::VCardFormat` type puts them behind the `Importer` / `Exporter` traits.

mod export;
mod import;
mod split;
mod uid;

pub use export::card_to_vcard;
pub use import::{VCARD_MEDIA_TYPE, card_from_vcard, import_vcards};
pub use split::split_vcards;
