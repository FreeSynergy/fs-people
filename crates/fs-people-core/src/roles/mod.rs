//! Roles of a person (ADR 0052): module map.
//!
//! Pattern: Facade (declarations) — a role is a described object, ours and yours alike; there is no
//! list of roles in the code. What a program ships is configuration (`known.json`).

mod assignment;
mod definition;
mod registry;

pub use assignment::RoleAssignment;
pub use definition::{AI_ROLE, RoleDefinition, RoleField, RoleScope};
pub use registry::RoleRegistry;
