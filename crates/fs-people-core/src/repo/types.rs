//! Small data types of the repository API.
//!
//! Pattern: Value objects — plain data for list filters and import results.

use fs_objects::ObjectId;

/// Filter for `PeopleRepo::list`.
#[derive(Clone, Debug, Default)]
pub struct ListFilter {
    /// Only this JSContact `kind`.
    pub kind: Option<String>,
    /// Only people holding this role, in any scope.
    pub role: Option<String>,
    /// Only cards with this substring in name, nickname, e-mail, phone or organisation.
    pub term: Option<String>,
    /// At most this many results.
    pub limit: Option<usize>,
    /// Skip this many results.
    pub offset: usize,
}

/// Result of an import.
#[derive(Clone, Debug, Default)]
pub struct ImportReport {
    /// Objects that were created.
    pub created: Vec<ObjectId>,
    /// `uid`s that already existed (not overwritten).
    pub skipped: Vec<String>,
    /// Entries that could not be converted or validated.
    pub failed: Vec<String>,
    /// Who was stored with a sensitive marking that names no category: treated as sensitive until
    /// someone clarifies it (ADR 0029). Name and `uid`.
    pub unclear: Vec<String>,
}

/// What `restore_role_config` did. Must be looked at: a stored file that could not be read is
/// skipped, not fatal, and only this says so.
#[derive(Clone, Debug, Default)]
#[must_use = "a stored role file may have been skipped: look at `skipped()`"]
pub struct RoleRestore(pub(super) Vec<String>);

impl RoleRestore {
    /// One line for each stored role file that was skipped, with the reason. Empty if all were read.
    pub fn skipped(&self) -> &[String] {
        &self.0
    }
}
