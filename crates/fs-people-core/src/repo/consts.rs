//! Constants of the contacts objects.
//!
//! Pattern: Registry (constants) — the AS2 types, context and media types used by this program in one place.

/// AS2 type of a contact envelope (ADR 0020).
pub const PROFILE_TYPE: &str = "Profile";
/// AS2 type of a confirmed same-person link.
pub const RELATIONSHIP_TYPE: &str = "Relationship";
/// `context` of all objects of this program.
pub const CONTEXT: &str = "contacts";
/// Media type of the card content.
pub const CONTENT_TYPE: &str = "application/jscontact+json";
pub(super) const AS2_TYPE: &str = "application/activity+json";
pub(super) const SAME_AS: &str = "owl:sameAs";
