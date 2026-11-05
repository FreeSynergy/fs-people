//! `fs-contacts-core`: program core of FreeSynergy contacts (no GUI).
//!
//! A contact is a JSContact `Card` (RFC 9553) in an AS2 `Profile` envelope stored through
//! `fs-store` (ADR 0014, 0019, 0020). See `DESIGN.md`.

pub mod card;
mod error;
pub mod index;
mod repo;
pub mod validate;
pub mod vcard;

pub use card::Card;
pub use error::{Error, Issue, Result};
pub use repo::{card_of, 
    ContactRepo, ImportReport, ListFilter, MatchKind, MatchReason, SameAsProposal, CONTEXT,
    CONTENT_TYPE, PROFILE_TYPE, RELATIONSHIP_TYPE,
};

/// Current time as an RFC 9553 `UTCDateTime`.
pub fn now_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
