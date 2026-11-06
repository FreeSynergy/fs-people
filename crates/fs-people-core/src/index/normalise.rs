//! Normalisation of e-mail addresses, phone numbers and names for matching.
//!
//! Pattern: Strategy (pure functions) — each `normalise_*` is the canonical form used both when
//! indexing and when querying, so the two sides always agree.

/// E-mail for matching: trimmed, `mailto:` removed, lowercase.
pub fn normalise_email(s: &str) -> String {
    let t = s.trim().to_lowercase();
    t.strip_prefix("mailto:").unwrap_or(&t).trim().to_string()
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
