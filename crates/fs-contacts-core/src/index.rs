//! Extracted index fields (ADR 0020) and normalisation.

use fs_store::IndexFields;

use crate::card::Card;

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
    let t = t.strip_prefix("tel:").or_else(|| t.strip_prefix("TEL:")).unwrap_or(t);
    let t = t.split(';').next().unwrap_or(t);
    let t = t.split(['x', 'X']).next().unwrap_or(t);
    let plus = t.trim_start().starts_with('+');
    let digits: String = t.chars().filter(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return String::new();
    }
    if plus {
        format!("+{digits}")
    } else if let Some(rest) = digits.strip_prefix("00") {
        format!("+{rest}")
    } else {
        digits
    }
}

/// Lowercased text for case-insensitive name/org matching.
pub fn fold(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Index fields of a card: `uid`, `kind`, `name_sort`, `email`, `phone`, `org`, `member`, `describes`.
pub fn index_fields(card: &Card) -> IndexFields {
    let mut f = IndexFields::new();
    let mut add = |k: &str, v: String| {
        if !v.is_empty() {
            let e = f.entry(k.to_string()).or_default();
            if !e.contains(&v) {
                e.push(v);
            }
        }
    };
    add("uid", card.uid.clone());
    add("kind", card.kind().to_string());
    add("name_sort", sort_key(card));
    for e in card.emails.values() {
        add("email", normalise_email(&e.address));
    }
    for p in card.phones.values() {
        add("phone", normalise_phone(&p.number));
    }
    for o in card.organizations.values() {
        if let Some(n) = &o.name {
            add("org", fold(n));
        }
    }
    for uid in card.members.keys() {
        add("member", uid.clone());
    }
    if let Some(d) = card.describes() {
        add("describes", d.to_string());
    }
    f
}

/// `surname, given` for ordered person names, otherwise the display name; lowercase.
fn sort_key(card: &Card) -> String {
    if let Some(n) = &card.name {
        let get = |k: &str| n.components.iter().find(|c| c.kind == k).map(|c| c.value.trim());
        if let (Some(sur), given) = (get("surname"), get("given")) {
            return fold(&format!("{sur}, {}", given.unwrap_or("")));
        }
    }
    fold(&card.display_name())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn email() {
        assert_eq!(normalise_email("  MailTo:Ada@Example.ORG "), "ada@example.org");
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
