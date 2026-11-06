//! Index fields extracted from a card (ADR 0020).
//!
//! Pattern: Adapter — projects the typed card onto the flat `IndexFields` map the store can query.

use fs_store::IndexFields;

use super::normalise::{fold, normalise_email, normalise_phone, normalise_uid};
use crate::card::Card;

/// Index fields of a card: `uid`, `kind`, `name_sort`, `email`, `phone`, `nickname`, `org`, `member`. (`describes` is an envelope field since fs-store 0.2.)
pub fn index_fields(card: &Card) -> IndexFields {
    let mut f = IndexFields::new();
    let mut add = |k: &str, v: String| {
        if !v.is_empty() {
            let e = f.entry(k.to_string()).or_default();
            if !e.contains(&v) {
                e.push(v);
            }
        }
    };
    add("uid", card.uid.clone());
    add("uid_key", normalise_uid(&card.uid));
    add("kind", card.kind().to_string());
    add("name_sort", sort_key(card));
    for e in card.emails.values() {
        add("email", normalise_email(&e.address));
    }
    for p in card.phones.values() {
        add("phone", normalise_phone(&p.number));
    }
    for n in card.nicknames.values() {
        add("nickname", fold(&n.name));
    }
    for o in card.organizations.values() {
        if let Some(n) = &o.name {
            add("org", fold(n));
        }
    }
    for uid in card.members.keys() {
        add("member", uid.clone());
    }
    f
}

/// `surname, given` for ordered person names, otherwise the display name; lowercase.
fn sort_key(card: &Card) -> String {
    if let Some(n) = &card.name {
        let get = |k: &str| {
            n.components
                .iter()
                .find(|c| c.kind == k)
                .map(|c| c.value.trim())
        };
        if let (Some(sur), given) = (get("surname"), get("given")) {
            return fold(&format!("{sur}, {}", given.unwrap_or("")));
        }
    }
    fold(&card.display_name())
}
