//! The wire form of the contract's values.
//!
//! Pattern: Adapter (declarations) — people and reports in one file, roles in the other.

mod person;
mod role;

pub(crate) use person::{
    count, read_card, read_filter, read_id, unpaged, write_ids, write_page, write_person,
    write_persons, write_proposals, write_relations, write_report,
};
pub(crate) use role::{
    read_definition, read_details, write_assignment, write_assignments, write_definition,
};
