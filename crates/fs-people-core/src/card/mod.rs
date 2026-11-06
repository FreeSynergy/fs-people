//! JSContact `Card` (RFC 9553): module map.
//!
//! Pattern: Facade — this module only declares the concept files and re-exports the model, so
//! callers keep writing `fs_people_core::card::Card`.

mod build;
mod display;
mod model;
mod parts;
mod relations;
mod types;
mod vendor;

pub use model::Card;
pub use parts::{
    Address, AddressComponent, EmailAddress, Link, Name, NameComponent, Nickname, Note,
    Organization, Phone, Title,
};
pub use types::{DESCRIBES_KEY, Extra, Map, SENSITIVE_KEY, VENDOR};
