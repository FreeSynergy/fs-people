//! Small data types of the repository API.
//!
//! Pattern: Value objects — plain data for list filters and import results.

use fs_store::ObjectId;

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
}
