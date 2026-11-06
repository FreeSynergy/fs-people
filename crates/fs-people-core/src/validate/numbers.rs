//! Numbers that plain JSON handling cannot keep exactly.
//!
//! Pattern: Specification over the text — a card's unknown properties and `details` are lossless, and
//! a number that `serde_json` would change cannot be. So it is refused where it comes in, while its text
//! is still there: an integer beyond `i64` / `u64`, a float whose value does not come back, and `-0`,
//! which a later read changes. (`1.10` stays: it is the value of `1.1`, only written differently.)

use serde_json::Value;

use crate::error::{Error, Result};

/// The first number in `text` that cannot be kept exactly.
#[must_use]
pub fn first_inexact(text: &str) -> Option<String> {
    tokens(text)
        .find(|t| !is_exact(t))
        .map(|t| t.chars().take(64).collect())
}

/// Refuse `text` if one of its numbers cannot be kept exactly.
pub fn check_exact_numbers(text: &str) -> Result<()> {
    first_inexact(text).map_or(Ok(()), |token| Err(inexact(&token)))
}

/// [`check_exact_numbers`] for the text of a card, where `-0` is for `create` and `update` to judge:
/// a card that already holds it can still be edited.
pub fn check_card_numbers(text: &str) -> Result<()> {
    let wrong = tokens(text).find(|t| !is_exact(t) && !is_negative_zero(t));
    wrong.map_or(Ok(()), |token| {
        Err(inexact(&token.chars().take(64).collect::<String>()))
    })
}

/// The numbers in a text that is valid JSON, or close to it.
fn tokens(text: &str) -> impl Iterator<Item = &str> {
    let bytes = text.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < bytes.len() {
            match bytes[i] {
                b'"' => i = after_string(bytes, i),
                b'-' | b'0'..=b'9' => {
                    let start = i;
                    while i < bytes.len()
                        && matches!(bytes[i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                    {
                        i += 1;
                    }
                    return Some(&text[start..i]);
                }
                _ => i += 1,
            }
        }
        None
    })
}

pub(crate) fn after_string(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() && bytes[i] != b'"' {
        i += if bytes[i] == b'\\' { 2 } else { 1 };
    }
    i + 1
}

/// The refusal of one number.
pub(crate) fn inexact(token: &str) -> Error {
    Error::InexactNumber(token.to_string())
}

fn is_negative_zero(token: &str) -> bool {
    token.starts_with('-') && serde_json::from_str::<f64>(token).is_ok_and(|value| value == 0.0)
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
pub fn has_negative_zero(value: &Value) -> bool {
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
    fn valid_doubles_come_back() {
        assert!(is_exact("13.049435775302797"));
        // An odd multiplier walks through all kinds of values; the same run every time.
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
        for _ in 0..200_000 {
            seed = seed.wrapping_mul(0x2545_F491_4F6C_DD1D).wrapping_add(1);
            let unit = f64::from(u32::try_from(seed >> 32).unwrap()) / f64::from(u32::MAX);
            let value =
                (unit - 0.5) * 180.0 + f64::from(u16::try_from(seed & 0xFFFF).unwrap()) * 1e-12;
            assert!(is_exact(&value.to_string()), "{value}");
        }
    }

    #[test]
    fn strings_are_not_numbers_and_a_card_may_hold_a_negative_zero_for_its_edit() {
        let text = r#"[{"a":"-0 \" 123456789012345678901234567890"},{"a":-0},{"b":[1,2]}]"#;
        assert_eq!(first_inexact(text).as_deref(), Some("-0"));
        assert!(check_exact_numbers(text).is_err());
        assert!(check_card_numbers(text).is_ok());
        assert!(check_card_numbers(r#"{"a":18446744073709551616}"#).is_err());
        assert!(check_exact_numbers(r#"{"a":[1.5,2],"b":"-0"}"#).is_ok());
    }
}
