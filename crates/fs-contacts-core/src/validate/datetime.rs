//! `UTCDateTime` syntax check of RFC 9553.
//!
//! Pattern: Specification — a pure predicate; no date arithmetic needed.

/// `YYYY-MM-DDTHH:MM:SS[.fff]Z` (RFC 9553 `UTCDateTime`).
pub fn is_utc_datetime(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20 || !s.ends_with('Z') {
        return false;
    }
    let digit = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let fixed = digit(0..4)
        && b[4] == b'-'
        && digit(5..7)
        && b[7] == b'-'
        && digit(8..10)
        && b[10] == b'T'
        && digit(11..13)
        && b[13] == b':'
        && digit(14..16)
        && b[16] == b':'
        && digit(17..19);
    if !fixed {
        return false;
    }
    let rest = &s[19..s.len() - 1];
    rest.is_empty()
        || (rest.starts_with('.')
            && rest.len() > 1
            && rest[1..].bytes().all(|c| c.is_ascii_digit()))
}
