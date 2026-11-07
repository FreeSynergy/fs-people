//! `fs-people-server`: the contract `people.capnp` over a Unix socket, in front of any provider of
//! `People`, `RoleBook` and `PeopleFeed`. It only translates: no people logic lives here.
//!
//! Interface status: draft (ADR 0045).
//!
//! Pattern: Facade — this file only declares modules and re-exports.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// The generated `Server` traits are async for every method, also those that never wait.
#![allow(clippy::missing_errors_doc, clippy::unused_async_trait_impl)]

mod errors;
mod facts;
mod feed;
mod formats;
mod people_service;
mod persons;
mod roles_service;
mod wire;

pub use people_service::{CONTRACT_VERSION, PeopleServer};
