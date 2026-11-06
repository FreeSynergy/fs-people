//! An edit never takes a sensitive marking away.
//!
//! Pattern: Memento — what the card was marked with before is compared with what it holds now; what an
//! edit removed is put back at card level. A marking that names no category stays just that, moved as
//! it came, unless the edit names categories: replacing "unclear" with what is meant is the point of
//! clarifying it.

use serde_json::Value;

use super::model::Card;
use super::types::SENSITIVE_KEY;

impl Card {
    /// Put back whatever `old` was marked with and this card no longer is.
    pub fn keep_marks_of(&mut self, old: &Card) {
        let named_now = self.sensitive_categories();
        let gone: Vec<Value> = old
            .sensitive_categories()
            .into_iter()
            .filter(|category| !named_now.contains(category))
            .map(Value::String)
            .collect();
        let gone_unclear = if named_now.is_empty() {
            let mut left = self.unclear_markings();
            let mut lost = Vec::new();
            for marking in old.unclear_markings() {
                match left.iter().position(|other| *other == marking) {
                    Some(found) => {
                        left.remove(found);
                    }
                    None => lost.push(marking),
                }
            }
            lost
        } else {
            Vec::new()
        };
        let level = self.extra.get(SENSITIVE_KEY).cloned();
        let value = match (level, gone.is_empty(), gone_unclear.as_slice()) {
            (_, true, []) => return,
            // Alone, it stays exactly what it was.
            (None, true, [one]) => one.clone(),
            (level, ..) => {
                let mut items = match level {
                    Some(Value::Array(items)) => items,
                    Some(other) => vec![other],
                    None => Vec::new(),
                };
                items.extend(gone);
                // Not a list of names any more, so it still reads as unclear.
                items.extend(gone_unclear);
                Value::Array(items)
            }
        };
        self.extra.insert(SENSITIVE_KEY.to_string(), value);
    }
}
