//! The [`Card`] data type: a typed JSContact card with lossless extras.
//!
//! Pattern: Newtype-style value object — plain serde data; behaviour lives in `build`,
//! `display` and `vendor` so this file stays a pure description of the wire form.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::parts::{Address, EmailAddress, Link, Name, Nickname, Note, Organization, Phone, Title};
use super::types::{Extra, Map};

fn is_empty_map<T>(m: &Map<T>) -> bool {
    m.is_empty()
}

/// A JSContact card (RFC 9553).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Card {
    /// JSON `@type`; must be `"Card"`.
    #[serde(rename = "@type", default)]
    pub type_: String,
    /// JSContact version; must be `"1.0"`.
    #[serde(default)]
    pub version: String,
    /// Globally unique id of the card (immutable once stored).
    #[serde(default)]
    pub uid: String,
    /// `individual` (default), `group`, `org`, `location`, `device`, `application` or
    /// `<domain>:<value>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Creation time (`UTCDateTime`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// Time of the last change (`UTCDateTime`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    /// The name of the entity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
    /// Nicknames by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub nicknames: Map<Nickname>,
    /// Organisations by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub organizations: Map<Organization>,
    /// Job titles by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub titles: Map<Title>,
    /// E-mail addresses by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub emails: Map<EmailAddress>,
    /// Phone numbers by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub phones: Map<Phone>,
    /// Postal addresses by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub addresses: Map<Address>,
    /// Links by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub links: Map<Link>,
    /// Notes by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub notes: Map<Note>,
    /// Group members: member card `uid` -> `true` (only for `kind: group`).
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub members: Map<bool>,
    /// RFC 9553 `relatedTo` (kept verbatim; not used for same-person links).
    #[serde(default, rename = "relatedTo", skip_serializing_if = "is_empty_map")]
    pub related_to: Map<Value>,
    /// Keywords, `keyword -> true`.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub keywords: Map<bool>,
    /// Everything else, including vendor properties `<domain>:<name>`.
    #[serde(flatten)]
    pub extra: Extra,
}
