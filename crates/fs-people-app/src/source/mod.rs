//! From the program's contract to the workbench's source.
//!
//! Pattern: Adapter — `adapter.rs` drives the async contract from the interface thread, `sheet.rs`
//! shows a card, `draft.rs` makes a form of a card, `apply.rs` writes a form back onto a card.

mod adapter;
mod apply;
mod draft;
mod marks;
mod sheet;

pub use adapter::PeopleAdapter;
pub(crate) use adapter::reason_text;
pub use apply::apply_draft;
pub(crate) use marks::is_marked;
