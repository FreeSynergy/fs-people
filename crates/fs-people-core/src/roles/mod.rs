//! Roles of a person (ADR 0052): module map.
//!
//! Pattern: Facade (declarations) — a role is a described object, ours and yours alike; there is no
//! list of roles in the code. What a program ships is configuration (`known.json`, a SKOS concept
//! scheme as JSON-LD).

mod assignment;
mod definition;
mod directory;
mod ident;
mod registry;
mod scheme;

pub use assignment::RoleAssignment;
pub use definition::{RoleDefinition, RoleField, RoleKind, RoleScope};
pub use directory::RoleDirectory;
pub use ident::{AI_ROLE, AI_ROLE_IRI, ROLE_BASE, full_id, is_ai_role, stored_forms};
pub use registry::RoleRegistry;
pub use scheme::RoleScheme;
