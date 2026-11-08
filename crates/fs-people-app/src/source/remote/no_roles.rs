//! A role book that says nothing.
//!
//! Pattern: Null object — the contract answered with "not possible" (ADR 0045).

use async_trait::async_trait;

/// A role book that has nothing to say: every method answers "not possible" (ADR 0045), so the
/// window shows no roles. Used with [`RemotePeople`] until the proxy asks the server for roles.
pub struct NoRoles;

#[async_trait]
impl fs_people_core::RoleBook for NoRoles {}
