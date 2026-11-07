//! One role file: a SKOS concept scheme written as JSON-LD.
//!
//! Pattern: Adapter — reads the JSON-LD form (`@context` with `@base`, `type`, `names`, `roles`) into
//! plain [`RoleDefinition`]s whose id is the full one (base + short id). Nothing is fetched: the
//! context file `roles.jsonld` does not exist yet, only `@base` is read from `@context`.

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;

use super::definition::{RoleDefinition, owner_id};
use super::ident::ROLE_BASE;
use crate::card::Map;
use crate::error::{Error, Issue, Result};

const SCHEME_TYPE: &str = "skos:ConceptScheme";

/// A checked role file: its base address and its roles, with full ids.
#[derive(Clone, Debug, PartialEq)]
pub struct RoleScheme {
    /// Base address of the file; the full id of a role is this plus its short id.
    pub base: String,
    /// Name of the scheme per language.
    pub names: Map<String>,
    /// The roles, each with its full id.
    pub roles: Vec<RoleDefinition>,
    /// Set for the old array form: it is added to the file of the same base instead of replacing it.
    pub merge: bool,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(rename = "@context")]
    context: Value,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    names: Map<String>,
    #[serde(default)]
    roles: Vec<RoleDefinition>,
}

fn invalid(path: &str, message: &str) -> Error {
    Error::Invalid(vec![Issue {
        path: path.into(),
        message: message.into(),
    }])
}

/// The `@base` of a context: an object, or the object inside an array.
fn base_of(context: &Value) -> Option<String> {
    let object = |v: &Value| v.get("@base").and_then(Value::as_str).map(str::to_owned);
    match context {
        Value::Array(items) => items.iter().find_map(object),
        other => object(other),
    }
}

impl RoleScheme {
    /// Read and check a role file that somebody typed or sent. The old form, an array of definitions
    /// with short ids, is read as a file of our own base that merges over the shipped roles
    /// (provisional, so a configuration from before keeps working).
    pub fn parse(json: &str) -> Result<Self> {
        Self::read(json, false)
    }

    /// Read a role file that was accepted once and stored. An owner that was a sentence is migrated
    /// to an id ("CRM (later)" -> "crm-later") instead of refused; everything else is checked.
    pub fn parse_stored(json: &str) -> Result<Self> {
        Self::read(json, true)
    }

    fn read(json: &str, stored: bool) -> Result<Self> {
        let value: Value = serde_json::from_str(json)?;
        if value.is_array() {
            let roles: Vec<RoleDefinition> = serde_json::from_value(value)?;
            return Self::build(ROLE_BASE.into(), Map::new(), roles, true, stored);
        }
        let raw: Raw = serde_json::from_value(value)?;
        if raw.kind != SCHEME_TYPE {
            return Err(invalid("/type", "a role file is a skos:ConceptScheme"));
        }
        let base = base_of(&raw.context)
            .filter(|b| b.contains(':') && (b.ends_with('/') || b.ends_with('#')))
            .ok_or_else(|| invalid("/@context", "needs an @base address ending in / or #"))?;
        Self::build(base, raw.names, raw.roles, false, stored)
    }

    fn build(
        base: String,
        names: Map<String>,
        mut roles: Vec<RoleDefinition>,
        merge: bool,
        stored: bool,
    ) -> Result<Self> {
        let mut seen = BTreeSet::new();
        for (i, def) in roles.iter_mut().enumerate() {
            let short = def.id.trim();
            if short.is_empty() || short.contains([':', '/', '#']) {
                let message = "a short id: not empty, without scheme, / or #";
                return Err(invalid(&format!("/roles/{i}/id"), message));
            }
            def.id = format!("{base}{short}");
            if stored {
                def.owner = owner_id(&def.owner);
            }
            def.check()?;
            if !seen.insert(def.id.clone()) {
                return Err(Error::Conflict(format!("role {} is defined twice", def.id)));
            }
        }
        Ok(Self {
            base,
            names,
            roles,
            merge,
        })
    }
}
