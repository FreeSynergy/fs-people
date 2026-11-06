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
        if let Ok(v) = serde_json::to_value(self) {
            collect_sensitive(&v, &mut out);
        }
        out.sort();
        out.dedup();
        out
    }

    /// Whether any part of the card is marked sensitive.
    pub fn is_sensitive(&self) -> bool {
        !self.sensitive_categories().is_empty()
    }
}

fn collect_sensitive(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(o) => {
            for (k, v) in o {
                if k == SENSITIVE_KEY {
                    match v {
                        Value::Array(a) => {
                            out.extend(a.iter().filter_map(|x| x.as_str().map(str::to_string)));
                        }
                        Value::String(s) => out.push(s.clone()),
                        _ => {}
                    }
                } else {
                    collect_sensitive(v, out);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| collect_sensitive(x, out)),
        _ => {}
    }
}
