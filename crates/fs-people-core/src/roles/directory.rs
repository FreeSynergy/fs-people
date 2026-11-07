//! What a place that knows role definitions answers.
//!
//! Pattern: Strategy (trait) — `PeopleRepo` holds an `Arc<dyn RoleDirectory>` (`with_roles`), so the
//! shipped registry, a decorator or a test double are interchangeable; `RoleRegistry` is the default.
//! Each role file is one scheme with its own base address; loading replaces only that one.

use std::collections::BTreeMap;

use super::definition::RoleDefinition;
use crate::error::Result;

/// Role definitions by full id, from any number of role files side by side.
pub trait RoleDirectory: Send + Sync {
    /// Check `json` (a JSON-LD concept scheme) without changing anything; answers every definition
    /// that would be in force afterwards, by full id.
    fn preview(&self, json: &str) -> Result<BTreeMap<String, RoleDefinition>>;
    /// Check `json` and swap its scheme in at once, replacing only the scheme with the same base
    /// address and leaving the others. On error nothing changes.
    fn load(&self, json: &str) -> Result<()>;
    /// Like `load`, for a file that was accepted once and stored: an old owner that was a sentence is
    /// migrated to an id instead of refused.
    fn load_stored(&self, json: &str) -> Result<()>;
    /// The definition with this id; the full id, or a short id for one of our roles.
    fn find(&self, id: &str) -> Option<RoleDefinition>;
    /// All definitions in force, by full id.
    fn all(&self) -> Vec<RoleDefinition>;
    /// The base addresses of the loaded schemes, sorted.
    fn bases(&self) -> Vec<String>;
}
