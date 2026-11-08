//! `PhoneNumber`: E.164, typed with the usual separators.
//!
//! Pattern: Value object — `+`, country code and number, 7 to 15 digits in all. Spaces, hyphens,
//! dots, slashes and brackets may separate what is typed; the stored text (`as_str`) is the plain
//! `+<digits>`, `typed` the text as the person wrote it. No country is guessed for a number without
//! `+` (it is a problem the person solves).

use super::{ContactValue, ValueProblem};

/// A valid international phone number.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhoneNumber {
    plain: String,
    typed: String,
}

impl PhoneNumber {
    /// The text as it was typed, separators included.
    pub fn typed(&self) -> &str {
        &self.typed
    }
}

impl ContactValue for PhoneNumber {
    fn parse(typed: &str) -> Result<Self, ValueProblem> {
        let typed = typed.trim();
        if typed.is_empty() {
            return Err(ValueProblem::Empty);
        }
        let Some(rest) = typed.strip_prefix('+') else {
            return Err(ValueProblem::NoCountryCode);
        };
        let mut digits = String::new();
        for c in rest.chars() {
            if c.is_ascii_digit() {
                digits.push(c);
            } else if !" -./()".contains(c) {
                return Err(ValueProblem::PhoneCharacter);
            }
        }
        if !(7..=15).contains(&digits.len()) || digits.starts_with('0') {
            return Err(ValueProblem::PhoneLength);
        }
        Ok(Self {
            plain: format!("+{digits}"),
            typed: typed.to_string(),
        })
    }

    fn as_str(&self) -> &str {
        &self.plain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separators_go_and_the_plain_form_stays() {
        let p = PhoneNumber::parse(" +49 (30) 1234-5678 ").unwrap();
        assert_eq!(p.as_str(), "+493012345678");
        assert_eq!(p.typed(), "+49 (30) 1234-5678");
    }

    #[test]
    fn each_rule_has_its_own_problem() {
        let cases = [
            ("", ValueProblem::Empty),
            ("030 123456", ValueProblem::NoCountryCode),
            ("+49 30 12a456", ValueProblem::PhoneCharacter),
            ("+49 30", ValueProblem::PhoneLength),
            ("+1234567890123456", ValueProblem::PhoneLength),
            ("+0301234567", ValueProblem::PhoneLength),
        ];
        for (typed, problem) in cases {
            assert_eq!(PhoneNumber::check(typed), Err(problem), "{typed:?}");
        }
    }
}
