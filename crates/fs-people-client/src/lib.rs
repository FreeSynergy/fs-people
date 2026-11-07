//! `fs-people-client`: the people program as a client of its own contract `people.capnp`.
//!
//! [`RpcPeople`] implements [`fs_people_core::People`] and [`fs_people_core::RoleBook`] over a Unix
//! socket, so a frontend depends on the traits and talks to a running `fs-people-server`, never to
//! the core of the program (ADR 0021). What a method does not offer answers the `Unsupported` of the
//! provider rule (ADR 0045); `DESIGN.md` lists which.
//!
//! Interface status: draft (ADR 0045), as the contract.
//!
//! Pattern: Facade — this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod peer;

pub use peer::RpcPeople;
