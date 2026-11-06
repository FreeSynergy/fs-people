//! The store-backed contact book: module map.
//!
//! Pattern: Facade (declarations) — `ContactRepo` is the one entry point; the files behind it
//! are one concept each (CRUD, search, groups, links, proposals, import / export).

mod backend;
mod book;
mod consts;
mod crud;
mod envelope;
mod find;
mod groups;
mod handle;
mod links;
mod proposals;
mod transfer;
mod types;

pub use consts::{CONTENT_TYPE, CONTEXT, PROFILE_TYPE, RELATIONSHIP_TYPE};
pub use envelope::card_of;
pub use handle::ContactRepo;
pub use types::{ImportReport, ListFilter};
