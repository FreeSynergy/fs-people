//! Vendor properties of a card: `describes` and the sensitive marking (ADR 0029).
//!
//! Pattern: Visitor (recursive walk) — `collect_sensitive` visits the serialised card and every
//! nested object, because the marking is allowed at any level.

use serde_json::Value;

use super::model::Card;
use super::types::{DESCRIBES_KEY, SENSITIVE_KEY};

impl Card {
    /// Vendor / unknown property.
    pub fn vendor(&self, key: &str) -> Option<&Value> {
        self.extra.get(key)
    }

    /// Set a vendor / unknown property.
    pub fn set_vendor(&mut self, key: &str, value: Value) {
        self.extra.insert(key.into(), value);
    }

    /// Actor IRI / DID this card describes (AS2 `Profile.describes`).
    pub fn describes(&self) -> Option<&str> {
        self.vendor(DESCRIBES_KEY).and_then(Value::as_str)
    }

    /// Mark the whole card as holding sensitive data of the given DPV categories (ADR 0029).
    pub fn mark_sensitive<I, S>(&mut self, categories: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let list: Vec<Value> = categories
            .into_iter()
            .map(|c| Value::String(c.into()))
            .collect();
        self.extra.insert(SENSITIVE_KEY.into(), Value::Array(list));
    }

    /// All DPV categories marked anywhere in the card (card level or any nested object),
    /// sorted, unique.
    pub fn sensitive_categories(&self) -> Vec<String> {
        let mut out = Vec::new();
        visit_markings(&self.to_value(), "", &mut |_, marking| {
            categories_of(marking, &mut out);
        });
        out.sort();
        out.dedup();
        out
    }

    /// Whether any part of the card carries the marking, also one that names no category: that
    /// fails closed.
    pub fn is_sensitive(&self) -> bool {
        let mut found = false;
        visit_markings(&self.to_value(), "", &mut |_, _| found = true);
        found
    }

    /// Where the card carries a marking that is not a list of category names.
    pub fn unclear_sensitive_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        visit_markings(&self.to_value(), "", &mut |path, marking| {
            if !is_clear(marking) {
                paths.push(path.to_string());
            }
        });
        paths
    }
}

fn categories_of(marking: &Value, out: &mut Vec<String>) {
    let mut one = |text: &str| {
        if !text.trim().is_empty() {
            out.push(text.to_string());
        }
    };
    match marking {
        Value::Array(list) => list.iter().filter_map(Value::as_str).for_each(&mut one),
        Value::String(text) => one(text),
        _ => {}
    }
}

fn is_clear(marking: &Value) -> bool {
    let named = |v: &Value| v.as_str().is_some_and(|t| !t.trim().is_empty());
    match marking {
        Value::Array(list) => !list.is_empty() && list.iter().all(named),
        other => named(other),
    }
}

/// Call `found` with the path and value of every marking in `value`.
fn visit_markings(value: &Value, path: &str, found: &mut dyn FnMut(&str, &Value)) {
    match value {
        Value::Object(members) => {
            for (key, member) in members {
                let here = format!("{path}/{key}");
                if key == SENSITIVE_KEY {
                    found(&here, member);
                } else {
                    visit_markings(member, &here, found);
                }
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                visit_markings(item, &format!("{path}/{i}"), found);
            }
        }
        _ => {}
    }
}
