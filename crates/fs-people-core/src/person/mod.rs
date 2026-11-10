//! What a person is as a data type: its description, patch, check and suggestions.
//!
//! Pattern: Facade — declares the concept files; the description is `impl DescribesType for Card`.

mod check;
mod describe;
mod fields;
mod patch;
mod suggest;

pub use check::check;
pub use describe::{PERSON, PROGRAM};
pub use patch::PatchObject;
pub use suggest::{SCAN_LIMIT, is_suggestible, suggest};
