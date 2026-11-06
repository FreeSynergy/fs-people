//! The wire form of the contract's values.
//!
//! Pattern: Adapter (declarations) — people, the report of an import and roles, one file each.

mod person;
mod report;
mod role;

pub(crate) use person::{
    count, ids_count, intake, proposals_count, read_card, read_filter, relations_count, required,
    unpaged, write_ids, write_page, write_person, write_persons, write_proposals, write_relations,
};
pub(crate) use report::write_report;
pub(crate) use role::{
    assignments_count, definitions_count, read_definition, read_details, write_assignment,
    write_assignments, write_definition,
};
