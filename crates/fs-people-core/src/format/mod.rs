//! Import / export formats behind traits: module map.
//!
//! Pattern: Adapter (declarations) — each format file wraps a foreign representation (vCard,
//! JSContact JSON) in the `Importer` / `Exporter` traits; more formats plug in the same way.

mod jscontact;
mod traits;
mod vcard;

pub use jscontact::JsContactFormat;
pub use traits::{Exporter, ImportItems, ImportedCard, Importer};
pub use vcard::VCardFormat;
