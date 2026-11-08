//! Constants of the people objects.
//!
//! Pattern: Registry (constants) — the AS2 types, context and media types used by this program in one place.

/// AS2 type of a person envelope (ADR 0020).
pub const PROFILE_TYPE: &str = "Profile";
/// AS2 type of a confirmed same-person link.
pub const RELATIONSHIP_TYPE: &str = "Relationship";
/// AS2 type of a refused same-person proposal (an AS2 `Reject` of the proposed link).
pub const REJECTION_TYPE: &str = "Reject";
/// `context` of all objects of this program.
pub const CONTEXT: &str = "people";
/// Media type of the card content.
pub const CONTENT_TYPE: &str = "application/jscontact+json";
/// Type of a role of your own in the store; AS2 has none.
pub const ROLE_DEFINITION_TYPE: &str = "RoleDefinition";
/// Type of the stored role configuration; AS2 has none.
pub(super) const ROLE_CONFIG_TYPE: &str = "RoleConfig";
pub(super) const AS2_TYPE: &str = "application/activity+json";
pub(super) const SAME_AS: &str = "owl:sameAs";
