//! Shared type aliases and vendor-property keys of the card model.
//!
//! Pattern: Newtype (alias form) — names the JSContact `Id[T]` map and the verbatim-extras map
//! once, so every sub-object spells them the same way.

use std::collections::BTreeMap;

use serde_json::Value;

/// `Id[T]` map of JSContact.
pub type Map<T> = BTreeMap<String, T>;
/// Unknown / vendor properties, preserved verbatim.
pub type Extra = BTreeMap<String, Value>;

/// Vendor prefix (ADR 0028): independent of the instance domain.
pub const VENDOR: &str = "freesynergy.net:";
/// Vendor property carrying DPV categories of sensitive data (ADR 0029). Allowed on the card and
/// on any nested object.
pub const SENSITIVE_KEY: &str = "freesynergy.net:sensitive";
/// Vendor property: actor IRI / DID the card describes (AS2 `Profile.describes`).
pub const DESCRIBES_KEY: &str = "freesynergy.net:describes";
