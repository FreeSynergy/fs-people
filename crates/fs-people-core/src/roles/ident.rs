//! Identity of a role: short id, base address, full id.
//!
//! Pattern: Newtype-like pure functions over text — the full id of a role is its file's base address
//! plus its short id (ADR 0052). A stored assignment may still hold the short id of one of our roles
//! (older data); these functions are the one place that knows both forms, so nothing else compares
//! role ids by hand.

/// Base address of the roles `fs-people` ships. Provisional: who controls the domain is not checked.
pub const ROLE_BASE: &str = "https://freesynergy.net/ns/roles/";

/// Short id of the role that marks an AI (ADR 0054).
pub const AI_ROLE: &str = "ai";

/// Full id of the role that marks an AI.
pub const AI_ROLE_IRI: &str = "https://freesynergy.net/ns/roles/ai";

/// The full id of `id`: an id with a scheme (`https:`) is one already, a short id belongs to our
/// base. Provisional: a short id means "ours", also for a role defined with `define_role`.
pub fn full_id(id: &str) -> String {
    if id.contains(':') {
        id.to_string()
    } else {
        format!("{ROLE_BASE}{id}")
    }
}

/// Every form `id` may have been stored in: the full id, and for one of our roles the short id of
/// older data.
pub fn stored_forms(id: &str) -> Vec<String> {
    let full = full_id(id);
    let short = full.strip_prefix(ROLE_BASE).filter(|s| !s.is_empty());
    let mut forms = vec![full.clone()];
    forms.extend(short.map(str::to_string));
    forms
}
