//! `fs-people-core`: program core of FreeSynergy people (no GUI).
//!
//! A person is a JSContact `Card` (RFC 9553) in an AS2 `Profile` envelope stored through
//! `fs-objects` (ADR 0014, 0019, 0020). Start with the [`People`] trait; [`PeopleRepo`] is
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

pub mod card;
mod clock;
mod error;
pub mod feed;
pub mod format;
pub mod index;
pub mod matching;
mod people;
mod repo;
mod role_book;
pub mod roles;
pub mod validate;
pub mod vcard;

pub use card::Card;
pub use clock::now_utc;
pub use error::{Error, Issue, MAX_CARD_BYTES, Result};
pub use feed::{PeopleFeed, PersonChange, PersonListener, Subscription};
pub use format::{Exporter, ImportedCard, Importer, JsContactFormat, VCardFormat};
pub use matching::{
    Candidate, MatchKind, MatchReason, MatchStrategy, SameAsProposal, SharedContactPoint,
};
pub use people::People;
pub use repo::{
    CONTENT_TYPE, CONTEXT, ImportReport, ListFilter, PROFILE_TYPE, PeopleRepo, RELATIONSHIP_TYPE,
    ROLE_DEFINITION_TYPE, card_of,
};
pub use role_book::RoleBook;
pub use roles::{AI_ROLE, RoleAssignment, RoleDefinition, RoleField, RoleRegistry, RoleScope};
