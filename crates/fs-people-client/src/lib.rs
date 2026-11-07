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
// A closure reads better than the full path of a generated reader type.
#![allow(
    clippy::module_name_repetitions,
    clippy::redundant_closure_for_method_calls
)]

mod calls;
mod offered;
mod pages;
mod peer;
mod people_impl;
mod problems;
#[cfg(test)]
mod problems_table;
mod read;
mod remote;
mod roles_impl;
mod walk;

pub use fs_interfaces::rpc::client::{CALL_TIMEOUT, CONNECT_TIMEOUT, Timeouts};
pub use pages::{Cursor, PersonPage, PersonPages};
pub use peer::RpcPeople;
pub use walk::PageWalk;
