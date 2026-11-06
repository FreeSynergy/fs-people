//! `Persons` interface: module map.
//!
//! Pattern: Adapter (declarations) — the dispatch of the contract in `service`, one file per concern
//! behind it, so no file passes the size limit (ADR 0043).

mod cards;
mod links;
mod lists;
mod service;

pub(crate) use service::PersonsImpl;
