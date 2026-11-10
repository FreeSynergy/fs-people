//! What a person is as a data type: its description, patch, check and suggestions.
//!
//! Pattern: Facade — declares the concept files; the description is `impl DescribesType for Card`.

mod check;
mod describe;
mod edit;
mod fields;
mod patch;
mod suggest;
mod sync;
mod values;

pub use check::check;
pub use describe::{PERSON, PROGRAM};
pub use edit::change;
pub use patch::PatchObject;
pub use suggest::{MAX_SUGGESTIONS, SCAN_LIMIT, is_suggestible, suggest};
pub use values::values_of;
