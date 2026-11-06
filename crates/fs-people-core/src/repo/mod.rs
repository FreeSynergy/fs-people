//! The store-backed people program: module map.
//!
//! Pattern: Facade (declarations) — `PeopleRepo` is the one entry point; the files behind it
//! are one concept each (CRUD, search, groups, links, proposals, import / export).

mod ai;
mod assign;
mod backend;
mod book;
mod consts;
mod crud;
mod envelope;
mod find;
mod groups;
mod handle;
mod links;
mod list_export;
mod proposals;
mod relations;
mod removal;
mod role_config;
mod role_facade;
mod role_rules;
mod roles;
mod transfer;
mod types;

pub use consts::{CONTENT_TYPE, CONTEXT, PROFILE_TYPE, RELATIONSHIP_TYPE, ROLE_DEFINITION_TYPE};
pub use envelope::card_of;
pub use handle::PeopleRepo;
pub use list_export::ListExport;
pub use types::{ImportReport, ListFilter};
