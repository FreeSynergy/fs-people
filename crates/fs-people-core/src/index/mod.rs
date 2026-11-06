//! Index fields and normalisation (ADR 0020): module map.
//!
//! Pattern: Facade — declares the concept files and re-exports the public functions.

mod fields;
mod normalise;

pub use fields::index_fields;
pub use normalise::{fold, normalise_email, normalise_phone, normalise_uid};
