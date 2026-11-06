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
        // `relatedTo` comes from outside and is kept verbatim: it may hold anything until edited here.
        if !entry.is_object() {
            *entry = json!({"@type": "Relation"});
        }
        if let Some(object) = entry.as_object_mut() {
            let kinds = object.entry("relation").or_insert_with(|| json!({}));
            if !kinds.is_object() {
                *kinds = json!({});
            }
            if let Some(kinds) = kinds.as_object_mut() {
                kinds.insert(relation.to_string(), Value::Bool(true));
            }
        }
    }

    /// Remove one relation. An empty `relation` removes an entry that names none. An entry stays as
    /// long as it holds another relation or a member of its own (`Relation` is open, RFC 9553).
    pub fn unrelate(&mut self, uid: &str, relation: &str) {
        let Some(entry) = self.related_to.get_mut(uid) else {
            return;
        };
        let kinds = entry.get_mut("relation").and_then(Value::as_object_mut);
        if relation.is_empty() {
            if kinds.is_none_or(|k| k.is_empty()) {
                self.related_to.remove(uid);
            }
            return;
        }
        let Some(kinds) = kinds else { return };
        if kinds.remove(relation).is_none() || !kinds.is_empty() {
            return;
        }
        let bare = entry
            .as_object()
            .is_some_and(|o| o.keys().all(|k| k == "@type" || k == "relation"));
        if bare {
            self.related_to.remove(uid);
        }
    }

    /// All relations as `(uid, relation)` pairs, in `uid` order. A related card that names no
    /// relation appears with an empty one: it is related, RFC 9553 only leaves open how.
    pub fn relations(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (uid, entry) in &self.related_to {
            let kinds = entry.get("relation").and_then(Value::as_object);
            let named: Vec<&String> = kinds
                .into_iter()
                .flatten()
                .filter(|(_, on)| on.as_bool() == Some(true))
                .map(|(k, _)| k)
                .collect();
            if named.is_empty() {
                out.push((uid.clone(), String::new()));
            }
            out.extend(named.into_iter().map(|k| (uid.clone(), k.clone())));
        }
        out
    }
}
