//! Relations of a card: what someone is to the person of this card.
//!
//! Pattern: Command (in-memory, no I/O) — small edits of JSContact `relatedTo` (RFC 9553 §2.1.8);
//! the key is the `uid` of the related card, the value says what it is to this card. Unknown
//! members of a `Relation` stay as they are.

use serde_json::{Value, json};

use super::model::Card;

impl Card {
    /// The card with `uid` is `relation` to this card (`friend`, `colleague`, `<domain>:<value>`).
    pub fn relate(&mut self, uid: &str, relation: &str) {
        let entry = self
            .related_to
            .entry(uid.to_string())
            .or_insert_with(|| json!({"@type": "Relation", "relation": {}}));
        if entry.get("relation").is_none_or(|r| !r.is_object()) {
            entry["relation"] = json!({});
        }
        entry["relation"][relation] = Value::Bool(true);
    }

    /// Remove one relation; a related card without any relation left is removed too.
    pub fn unrelate(&mut self, uid: &str, relation: &str) {
        let Some(entry) = self.related_to.get_mut(uid) else {
            return;
        };
        let empty = entry
            .get_mut("relation")
            .and_then(Value::as_object_mut)
            .is_none_or(|m| {
                m.remove(relation);
                m.is_empty()
            });
        if empty {
            self.related_to.remove(uid);
        }
    }

    /// All relations as `(uid, relation)` pairs, in `uid` order.
    pub fn relations(&self) -> Vec<(String, String)> {
        self.related_to
            .iter()
            .flat_map(|(uid, v)| {
                let keys = v.get("relation").and_then(Value::as_object);
                keys.into_iter()
                    .flatten()
                    .filter(|(_, on)| on.as_bool() == Some(true))
                    .map(|(k, _)| (uid.clone(), k.clone()))
            })
            .collect()
    }
}
