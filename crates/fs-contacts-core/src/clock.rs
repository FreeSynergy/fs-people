//! The current time in the RFC 9553 `UTCDateTime` form.
//!
//! Pattern: none (pure helper) — one place for the timestamp format used by cards and the repository.

/// Current time as an RFC 9553 `UTCDateTime`.
pub fn now_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
