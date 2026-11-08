//! How a card is printed.
//!
//! Pattern: none (pure functions of a JSContact text).

use serde_json::Value;

/// Control characters in a value (a newline, a terminal escape) are shown as `?`.
fn clean(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

/// `id  name  first e-mail`, columns split by two spaces; a missing member is `-`.
pub(crate) fn line(id: &str, jscontact: &str) -> String {
    let v: Value = serde_json::from_str(jscontact).unwrap_or(Value::Null);
    let name = v
        .pointer("/name/full")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let email = v
        .get("emails")
        .and_then(Value::as_object)
        .and_then(|m| m.values().next())
        .and_then(|e| e.get("address"))
        .and_then(Value::as_str)
        .unwrap_or("-");
    format!("{id}  {}  {}", clean(name), clean(email))
}

/// The JSContact text pretty-printed; the text as it is when it is no JSON.
pub(crate) fn pretty(jscontact: &str) -> String {
    serde_json::from_str::<Value>(jscontact)
        .and_then(|v| serde_json::to_string_pretty(&v))
        .unwrap_or_else(|_| jscontact.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_has_three_columns_and_dashes_for_gaps() {
        let c = r#"{"name":{"full":"Ada"},"emails":{"e1":{"address":"a@x.org"}}}"#;
        assert_eq!(line("1", c), "1  Ada  a@x.org");
        assert_eq!(line("2", "not json"), "2  -  -");
    }
}
