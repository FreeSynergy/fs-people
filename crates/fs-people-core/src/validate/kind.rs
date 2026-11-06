//! The `kind` vocabulary of RFC 9553.
//!
//! Pattern: Specification — a pure predicate over the registered kinds and vendor values.

/// `kind` values of RFC 9553 section 2.1.4.
pub const KINDS: [&str; 6] = [
    "individual",
    "group",
    "org",
    "location",
    "device",
    "application",
];

/// `kind` is one of the registered values or a vendor value `<domain>:<value>`.
pub fn is_valid_kind(k: &str) -> bool {
    KINDS.contains(&k)
        || matches!(k.split_once(':'), Some((d, v)) if !d.is_empty() && !v.is_empty() && d.contains('.'))
}
