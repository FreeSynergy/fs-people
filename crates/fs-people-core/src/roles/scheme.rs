//! One role file: a SKOS concept scheme written as JSON-LD.
//!
//! Pattern: Adapter — reads the JSON-LD form (`@context` with `@base`, `type`, `names`, `roles`) into
//! plain [`RoleDefinition`]s whose id is the full one (base + short id). Nothing is fetched: the
//! context file `roles.jsonld` does not exist yet, only `@base` is read from `@context`.

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;

use super::definition::RoleDefinition;
use super::ident::{ROLE_BASE, full_id};
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
    /// Read and check a role file. The old form, an array of definitions, is read as a file of our
    /// own base (provisional, so a stored or sent configuration from before keeps working).
    pub fn parse(json: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(json)?;
        if value.is_array() {
            let roles: Vec<RoleDefinition> = serde_json::from_value(value)?;
            return Self::build(ROLE_BASE.into(), Map::new(), roles, false);
        }
        let raw: Raw = serde_json::from_value(value)?;
        if raw.kind != SCHEME_TYPE {
            return Err(invalid("/type", "a role file is a skos:ConceptScheme"));
        }
        let base = base_of(&raw.context)
            .filter(|b| b.contains(':') && (b.ends_with('/') || b.ends_with('#')))
            .ok_or_else(|| invalid("/@context", "needs an @base address ending in / or #"))?;
        Self::build(base, raw.names, raw.roles, true)
    }

    fn build(
        base: String,
        names: Map<String>,
        mut roles: Vec<RoleDefinition>,
        short: bool,
    ) -> Result<Self> {
        let mut seen = BTreeSet::new();
        for (i, def) in roles.iter_mut().enumerate() {
            if short && (def.id.trim().is_empty() || def.id.contains(':')) {
                return Err(invalid(
                    &format!("/roles/{i}/id"),
                    "a short id, without a scheme",
                ));
            }
            def.id = if short {
                format!("{base}{}", def.id)
            } else {
                full_id(&def.id)
            };
            def.check()?;
            if !seen.insert(def.id.clone()) {
                return Err(Error::Conflict(format!("role {} is defined twice", def.id)));
            }
        }
        Ok(Self { base, names, roles })
    }
}
