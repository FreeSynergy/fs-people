//! Constructors and mutators of [`Card`].
//!
//! Pattern: Builder (convenience form) — `Card::individual(..)` plus `add_email(..)` etc. build
//! a valid card step by step; ids are the next free `e1`, `p1`, `o1`, ...

use serde_json::Value;

use super::model::Card;
use super::parts::{EmailAddress, Name, Organization, Phone};
use super::types::Map;
use crate::clock::now_utc;

impl Card {
    /// A new valid card with a fresh `urn:uuid:` UUIDv7 `uid` and `name.full`.
    pub fn new(kind: &str, full_name: impl Into<String>) -> Self {
        let now = now_utc();
        Card {
            type_: "Card".into(),
            version: "1.0".into(),
            uid: format!("urn:uuid:{}", fs_store::ObjectId::new()),
            kind: Some(kind.into()),
            created: Some(now.clone()),
            updated: Some(now),
            name: Some(Name {
                full: Some(full_name.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// A new person card (`kind: individual`).
    pub fn individual(full_name: impl Into<String>) -> Self {
        Self::new("individual", full_name)
    }

    /// A new group card (`kind: group`), usable as folder / mailing list (ADR 0037).
    pub fn group(full_name: impl Into<String>) -> Self {
        Self::new("group", full_name)
    }

    /// `kind`, defaulting to `individual`.
    pub fn kind(&self) -> &str {
        self.kind.as_deref().unwrap_or("individual")
    }

    /// Whether this card is a group.
    pub fn is_group(&self) -> bool {
        self.kind() == "group"
    }

    /// Add an e-mail address under the next free id; returns the id.
    pub fn add_email(&mut self, address: impl Into<String>) -> String {
        let id = next_id(&self.emails, "e");
        let item = EmailAddress {
            type_: Some("EmailAddress".into()),
            address: address.into(),
            ..Default::default()
        };
        self.emails.insert(id.clone(), item);
        id
    }

    /// Add a phone number under the next free id; returns the id.
    pub fn add_phone(&mut self, number: impl Into<String>) -> String {
        let id = next_id(&self.phones, "p");
        let item = Phone {
            type_: Some("Phone".into()),
            number: number.into(),
            ..Default::default()
        };
        self.phones.insert(id.clone(), item);
        id
    }

    /// Add an organisation under the next free id; returns the id.
    pub fn add_organization(&mut self, name: impl Into<String>) -> String {
        let id = next_id(&self.organizations, "o");
        let item = Organization {
            type_: Some("Organization".into()),
            name: Some(name.into()),
            ..Default::default()
        };
        self.organizations.insert(id.clone(), item);
        id
    }

    /// The card as a JSON value (the stored content).
    ///
    /// # Panics
    /// Never in practice: the card contains only JSON-compatible data.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("Card serialises")
    }
}

fn next_id<T>(m: &Map<T>, prefix: &str) -> String {
    (1..=u32::MAX)
        .map(|i| format!("{prefix}{i}"))
        .find(|k| !m.contains_key(k))
        .unwrap()
}
