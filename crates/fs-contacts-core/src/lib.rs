//! `fs-contacts-core`: program core of FreeSynergy contacts (no GUI).
//!
//! A contact is a JSContact `Card` (RFC 9553) in an AS2 `Profile` envelope stored through
//! `fs-store` (ADR 0014, 0019, 0020). Start with the [`ContactBook`] trait; [`ContactRepo`] is
//! its store-backed implementation. See `DESIGN.md` for the pattern map.
//!
//! Pattern: Facade — this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// Pedantic lints that would only add noise to this crate's small, documented API:
// every fallible fn returns the crate `Result` (documented once on `Error`), and results of
// pure getters are used by callers anyway.
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions
)]

mod book;
pub mod card;
mod clock;
mod error;
pub mod format;
pub mod index;
pub mod matching;
mod repo;
pub mod validate;
pub mod vcard;

pub use book::ContactBook;
pub use card::Card;
pub use clock::now_utc;
pub use error::{Error, Issue, Result};
pub use format::{Exporter, ImportedCard, Importer, JsContactFormat, VCardFormat};
pub use matching::{
    Candidate, MatchKind, MatchReason, MatchStrategy, SameAsProposal, SharedContactPoint,
};
pub use repo::{
    CONTENT_TYPE, CONTEXT, ContactRepo, ImportReport, ListFilter, PROFILE_TYPE, RELATIONSHIP_TYPE,
    card_of,
};
