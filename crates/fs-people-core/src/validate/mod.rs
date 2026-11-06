//! Validation of a card against the required parts of RFC 9553: module map.
//!
//! Pattern: Facade — declares the rule files and re-exports `validate`, `issues` and helpers.

mod check;
mod datetime;
mod kind;
mod numbers;
mod sink;
mod sub_objects;

pub use check::{issues, validate};
pub use datetime::is_utc_datetime;
pub use kind::{KINDS, is_valid_kind};
pub(crate) use numbers::inexact;
pub use numbers::{check_exact_numbers, inexact_numbers};
