//! Numbers that plain JSON handling cannot keep exactly.
//!
//! Pattern: Specification over the text — a card's unknown properties and `details` are lossless, and
//! a number that `serde_json` would change cannot be. So it is refused where it comes in, while its text
//! is still there: an integer beyond `i64` / `u64`, a float whose value does not come back, and `-0`,
//! which a later read changes. (`1.10` stays: it is the value of `1.1`, only written differently.)

use serde_json::Value;

use crate::error::{Error, Result};

/// Each item of `text` that holds such a number, with the first of them. The items of a top-level
/// array count from 0; anything else is item 0.
#[must_use]
pub fn inexact_numbers(text: &str) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let top_array = text.trim_start().starts_with('[');
    let (mut i, mut depth, mut item) = (0, 0usize, 0);
    let mut found: Vec<(usize, String)> = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth = depth.saturating_sub(1),
            b',' if depth == 1 && top_array => item += 1,
            b'-' | b'0'..=b'9' => {
                let start = i;
                while i < bytes.len()
                    && matches!(bytes[i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    i += 1;
                }
                let token = &text[start..i];
                if !is_exact(token) && found.last().is_none_or(|(at, _)| *at != item) {
                    found.push((item, token.chars().take(64).collect()));
                }
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    found
}

/// Refuse `text` if one of its numbers cannot be kept exactly.
pub fn check_exact_numbers(text: &str) -> Result<()> {
    match inexact_numbers(text).into_iter().next() {
        None => Ok(()),
        Some((_, token)) => Err(inexact(&token)),
    }
}

/// The refusal of one number.
pub(crate) fn inexact(token: &str) -> Error {
    Error::InexactNumber(token.to_string())
}

fn is_exact(token: &str) -> bool {
    if !token.contains(['.', 'e', 'E']) {
        return token != "-0" && (token.parse::<i64>().is_ok() || token.parse::<u64>().is_ok());
    }
    // The parser that will read it: a value it gets wrong does not come back either.
    let Ok(value) = serde_json::from_str::<f64>(token) else {
        return false;
    };
    let negative_zero = value == 0.0 && token.starts_with('-');
    !negative_zero && decimal(token) == decimal(&format!("{value:e}"))
}

/// Digits without outer zeros and the power of ten they are to be multiplied by.
fn decimal(text: &str) -> (String, i64) {
    let unsigned = text.trim_start_matches('-');
    let (mantissa, exponent) = unsigned
        .split_once(['e', 'E'])
        .map_or((unsigned, 0), |(m, e)| {
            (m, e.parse().unwrap_or(i64::MAX / 2))
        });
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    let trimmed = digits.trim_end_matches('0');
    let dropped = i64::try_from(digits.len() - trimmed.len()).unwrap_or(0);
    let places = i64::try_from(fraction.len()).unwrap_or(0);
    let significant = trimmed.trim_start_matches('0').to_string();
    if significant.is_empty() {
        return (significant, 0);
    }
    (
        significant,
        exponent.saturating_sub(places).saturating_add(dropped),
    )
}

/// Whether a value holds a negative zero, which a card made in code can carry.
pub(super) fn has_negative_zero(value: &Value) -> bool {
    match value {
        Value::Number(n) => n.as_f64().is_some_and(|f| f == 0.0 && f.is_sign_negative()),
        Value::Array(items) => items.iter().any(has_negative_zero),
        Value::Object(members) => members.values().any(has_negative_zero),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_kept_and_what_is_not() {
        for good in [
            "0",
            "7",
            "-7",
            "18446744073709551615",
            "-9223372036854775808",
            "1.5",
            "1.10",
            "0.0",
            "1e5",
            "1E+2",
            "2.5e-3",
            "100.0",
        ] {
            assert!(is_exact(good), "{good}");
        }
        for bad in [
            "-0",
            "-0.0",
            "-0e1",
            "18446744073709551616",
            "-9223372036854775809",
            "123456789012345678901234567890",
            "1.2345678901234567890",
            "1e400",
            "1e-400",
        ] {
            assert!(!is_exact(bad), "{bad}");
        }
    }

    #[test]
    fn items_are_counted_and_strings_are_not_numbers() {
        let text = r#"[{"a":"-0 \" 123456789012345678901234567890"},{"a":-0},{"b":[1,2]},{"c":18446744073709551616,"d":-0}]"#;
        let found = inexact_numbers(text);
        assert_eq!(
            found,
            [
                (1, "-0".to_string()),
                (3, "18446744073709551616".to_string())
            ]
        );
        assert_eq!(
            inexact_numbers(r#"{"a":1,"b":-0}"#),
            [(0, "-0".to_string())]
        );
        assert!(check_exact_numbers(r#"{"a":[1.5,2],"b":"-0"}"#).is_ok());
    }
}
