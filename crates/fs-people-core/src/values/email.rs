//! `EmailAddress`: the practical rule of the HTML "valid e-mail address", with international domains.
//!
//! Pattern: Value object — the local part follows the HTML rule (atext characters, dots inside), the
//! domain is a host name with at least one dot, labels of 1 to 63 letters, digits or hyphens, not
//! starting or ending with a hyphen. A Unicode domain is accepted and kept in the form that was
//! typed; `ascii` gives the form that goes on the wire (IDNA, punycode).

use super::{ContactValue, ValueProblem};

/// A valid e-mail address. Equal addresses differ only in the case of the domain.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EmailAddress {
    text: String,
    ascii: String,
}

impl EmailAddress {
    /// The address with the domain in ASCII (`user@xn--bcher-kva.example`).
    pub fn ascii(&self) -> &str {
        &self.ascii
    }
}

fn atext(c: char) -> bool {
    c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~-".contains(c)
}

fn local_ok(local: &str) -> Result<(), ValueProblem> {
    if local.is_empty() {
        return Err(ValueProblem::EmptyLocal);
    }
    if local.chars().count() > 64 {
        return Err(ValueProblem::LocalTooLong);
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return Err(ValueProblem::LocalCharacter);
    }
    if local.chars().any(|c| c != '.' && !atext(c)) {
        return Err(ValueProblem::LocalCharacter);
    }
    Ok(())
}

fn label_ok(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn domain_ascii(domain: &str) -> Result<String, ValueProblem> {
    if domain.is_empty() {
        return Err(ValueProblem::EmptyDomain);
    }
    let ascii = idna::domain_to_ascii(domain).map_err(|_| ValueProblem::BadDomain)?;
    if ascii.len() > 253 || !ascii.split('.').all(label_ok) {
        return Err(ValueProblem::BadDomain);
    }
    if !ascii.contains('.') {
        return Err(ValueProblem::DomainWithoutDot);
    }
    Ok(ascii)
}

impl ContactValue for EmailAddress {
    fn parse(typed: &str) -> Result<Self, ValueProblem> {
        let typed = typed.trim();
        if typed.is_empty() {
            return Err(ValueProblem::Empty);
        }
        let mut parts = typed.split('@');
        let (Some(local), Some(domain), rest) = (parts.next(), parts.next(), parts.next()) else {
            return Err(ValueProblem::MissingAt);
        };
        if rest.is_some() {
            return Err(ValueProblem::ManyAt);
        }
        local_ok(local)?;
        let ascii_domain = domain_ascii(domain)?;
        Ok(Self {
            text: format!("{local}@{domain}"),
            ascii: format!("{local}@{ascii_domain}"),
        })
    }

    fn as_str(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_plus_addresses_are_valid() {
        for ok in [
            "ada@example.org",
            "a.b+tag@mail.example.co.uk",
            " x_y@a-b.de ",
        ] {
            assert!(EmailAddress::check(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn each_rule_has_its_own_problem() {
        let cases = [
            ("", ValueProblem::Empty),
            ("ada", ValueProblem::MissingAt),
            ("a@b@c.de", ValueProblem::ManyAt),
            ("@example.org", ValueProblem::EmptyLocal),
            ("ada@", ValueProblem::EmptyDomain),
            ("a b@example.org", ValueProblem::LocalCharacter),
            (".a@example.org", ValueProblem::LocalCharacter),
            ("a..b@example.org", ValueProblem::LocalCharacter),
            ("ada@localhost", ValueProblem::DomainWithoutDot),
            ("ada@-bad.example", ValueProblem::BadDomain),
            ("ada@exa mple.org", ValueProblem::BadDomain),
        ];
        for (typed, problem) in cases {
            assert_eq!(EmailAddress::check(typed), Err(problem), "{typed:?}");
        }
        let long = format!("{}@example.org", "a".repeat(65));
        assert_eq!(EmailAddress::check(&long), Err(ValueProblem::LocalTooLong));
    }

    #[test]
    fn an_international_domain_is_kept_as_typed_and_has_an_ascii_form() {
        let a = EmailAddress::parse("ada@bücher.example").unwrap();
        assert_eq!(a.as_str(), "ada@bücher.example");
        assert_eq!(a.ascii(), "ada@xn--bcher-kva.example");
    }
}
