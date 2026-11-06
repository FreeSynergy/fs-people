//! The items of a JSON array, before anything reads them.
//!
//! Pattern: Iterator-style scan — one item that no parser can read stays one failure among the others.

use crate::validate::after_string;

/// The text of each item of a top-level array, without reading them, so one item no parser can read
/// stays one failure. `None` if `text` is no such array, or does not end where the array ends.
#[must_use]
pub fn array_items(text: &str) -> Option<Vec<&str>> {
    let bytes = text.as_bytes();
    let open = text.len() - text.trim_start().len();
    if bytes.get(open) != Some(&b'[') {
        return None;
    }
    let (mut i, mut depth, mut start) = (open + 1, 1usize, open + 1);
    let mut items = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i = after_string(bytes, i);
                continue;
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b',' if depth == 1 => {
                items.push(text.get(start..i)?);
                start = i + 1;
            }
            _ => {}
        }
        if depth == 0 {
            let last = text.get(start..i)?;
            if !(items.is_empty() && last.trim().is_empty()) {
                items.push(last);
            }
            return text[i + 1..].trim().is_empty().then_some(items);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_of_an_array_stay_apart() {
        assert_eq!(
            array_items(r#" [{"a":"],"},[1,2], 1e400] "#).unwrap(),
            [r#"{"a":"],"}"#, "[1,2]", " 1e400"]
        );
        assert_eq!(array_items("[]").unwrap(), Vec::<&str>::new());
        assert!(array_items("[1,").is_none());
        assert!(array_items("[1] x").is_none());
        assert!(array_items("{}").is_none());
        assert!(array_items("[").is_none());
    }
}
