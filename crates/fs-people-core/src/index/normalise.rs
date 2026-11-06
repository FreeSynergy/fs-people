//! Normalisation of e-mail addresses, phone numbers and names for matching.
//!
//! Pattern: Strategy (pure functions) — each `normalise_*` is the canonical form used both when
//! indexing and when querying, so the two sides always agree.

/// E-mail for matching: trimmed, `mailto:` removed, lowercase.
pub fn normalise_email(s: &str) -> String {
    let mut t = s.trim().to_lowercase();
    while let Some(rest) = t.strip_prefix("mailto:") {
        t = rest.trim_start().to_string();
    }
    t
}

/// The identity a card's `uid` stands for: the same UUID however it is written (with or without
/// `urn:uuid:`, in any case, with or without hyphens) gives one text; any other `uid` stays as it is.
pub fn normalise_uid(uid: &str) -> String {
    let t = uid.trim();
    let bare = match t.get(..9) {
        Some(head) if head.eq_ignore_ascii_case("urn:uuid:") => &t[9..],
        _ => t,
    };
    let hex: String = bare.chars().filter(|c| *c != '-').collect();
    let hyphens_fit = bare.len() == 32
        || (bare.len() == 36 && [8, 13, 18, 23].iter().all(|i| bare.as_bytes()[*i] == b'-'));
    if !hyphens_fit || hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return t.to_string();
    }
    let hex = hex.to_ascii_lowercase();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// Phone number for matching, E.164-ish: `tel:` and visual separators removed; a leading `+` or
/// `00` gives `+<digits>`, otherwise just the digits (no country is guessed). Extensions
/// (`;ext=...`, `x123`) are cut off. Empty if there are no digits.
pub fn normalise_phone(s: &str) -> String {
    let t = s.trim();
    let t = t
        .strip_prefix("tel:")
        .or_else(|| t.strip_prefix("TEL:"))
        .unwrap_or(t);
    let t = t.split(';').next().unwrap_or(t);
    let t = t.split(['x', 'X']).next().unwrap_or(t);
    let plus = t.trim_start().starts_with('+');
    let digits: String = t.chars().filter(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return String::new();
    }
    if plus {
        format!("+{digits}")
    } else if let Some(rest) = digits.strip_prefix("00").filter(|r| !r.is_empty()) {
        format!("+{rest}")
    } else {
        digits
    }
}

/// Lowercased text for case-insensitive name/org matching.
pub fn fold(s: &str) -> String {
    s.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn email() {
        assert_eq!(
            normalise_email("  MailTo:Ada@Example.ORG "),
            "ada@example.org"
        );
    }

    #[test]
    fn email_twice() {
        assert_eq!(normalise_email("mailto:mailto:A@x.org"), "a@x.org");
    }

    #[test]
    fn uid_is_one_identity_however_written() {
        let plain = "01a1aaef-ba80-7f7a-8dff-0000000000ab";
        for spelling in [
            "01a1aaef-ba80-7f7a-8dff-0000000000ab",
            "urn:uuid:01a1aaef-ba80-7f7a-8dff-0000000000ab",
            "URN:UUID:01A1AAEF-BA80-7F7A-8DFF-0000000000AB",
            "01a1aaefba807f7a8dff0000000000ab",
        ] {
            assert_eq!(normalise_uid(spelling), plain, "{spelling}");
        }
        assert_eq!(normalise_uid(" Plain-UID "), "Plain-UID");
        assert_eq!(normalise_uid("urn:uuid:not a uuid"), "urn:uuid:not a uuid");
    }

    #[test]
    fn phone() {
        for (i, o) in [
            ("+49 (30) 123-4567", "+49301234567"),
            ("0049 30 1234567", "+49301234567"),
            ("tel:+49-30-1234567;ext=12", "+49301234567"),
            ("030/1234567", "0301234567"),
            ("+1 555 0100 x23", "+15550100"),
            ("n/a", ""),
            ("00", "00"),
        ] {
            assert_eq!(normalise_phone(i), o, "{i}");
        }
    }

    proptest! {
        #[test]
        fn normalisation_is_idempotent(s in "\\PC{0,30}") {
            let p = normalise_phone(&s);
            prop_assert_eq!(normalise_phone(&p), p);
            let e = normalise_email(&s);
            prop_assert_eq!(normalise_email(&e), e);
        }
    }
}
