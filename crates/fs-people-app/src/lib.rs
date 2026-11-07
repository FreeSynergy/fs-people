//! `fs-people-app`: the frontend of the people program (ADR 0005, 0009, 0037).
//!
//! Arguments, the words, and an adapter from the program's contract to the
//! workbench's `PeopleSource`. It imports no `blinc_*` crate: the window and the work surface come
//! from `fs-components`. The data comes from a running `fs-people-server` over its Unix socket
//! (`fs-people-client`); the app holds no store (see `DESIGN.md`).
//!
//! Pattern: none — this file only declares modules and re-exports the public API (no behaviour).

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate, clippy::module_name_repetitions)]

pub mod args;
mod source;
mod surface;
mod transfer;
mod words;

pub use source::{PeopleAdapter, apply_draft};
pub use surface::{PeopleScreen, app_widget};
pub use transfer::{ExportReport, ImportSummary, export_all, import_file};
pub use words::{cannot_reach, labels, no_socket, not_exported, wrote};
