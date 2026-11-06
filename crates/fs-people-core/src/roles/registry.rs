//! The role definitions loaded as configuration.
//!
//! Pattern: Registry, swapped atomically like a config plugin (ADR 0045, learned from Zentinel) —
//! a new configuration is checked completely first; readers see the old or the new one, never a mix.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use super::definition::RoleDefinition;
use crate::error::{Error, Result};
use crate::validate::check_exact_numbers;

/// What this program ships. Data, not code.
const KNOWN: &str = include_str!("known.json");

type Definitions = Arc<BTreeMap<String, RoleDefinition>>;

/// Role definitions from configuration. Cheap to clone; clones share the same definitions.
#[derive(Clone, Debug)]
pub struct RoleRegistry(Arc<RwLock<Definitions>>);

impl RoleRegistry {
    /// The roles this program ships.
    ///
    /// # Panics
    /// Never in a released build: `known.json` is part of the crate and covered by tests.
    pub fn known() -> Self {
        Self(Arc::new(RwLock::new(
            parse(KNOWN).expect("known.json is valid"),
        )))
    }

    /// An empty registry.
    pub fn empty() -> Self {
        Self(Arc::new(RwLock::new(Arc::default())))
    }

    /// Check `json` (an array of definitions) without changing anything; answers what it defines.
    pub fn check(json: &str) -> Result<Arc<BTreeMap<String, RoleDefinition>>> {
        parse(json)
    }

    /// Check `json` (an array of definitions) and swap it in at once. On error nothing changes.
    ///
    /// # Panics
    /// Only if another thread panicked while swapping, which holds no user code.
    pub fn load(&self, json: &str) -> Result<()> {
        let next = parse(json)?;
        *self.0.write().expect("role registry lock") = next;
        Ok(())
    }

    /// The definitions in force now.
    ///
    /// # Panics
    /// Only if another thread panicked while swapping, which holds no user code.
    pub fn snapshot(&self) -> Arc<BTreeMap<String, RoleDefinition>> {
        self.0.read().expect("role registry lock").clone()
    }
}

fn parse(json: &str) -> Result<Definitions> {
    check_exact_numbers(json)?;
    let list: Vec<RoleDefinition> = serde_json::from_str(json)?;
    let mut map = BTreeMap::new();
    for def in list {
        def.check()?;
        let id = def.id.clone();
        if map.insert(id.clone(), def).is_some() {
            return Err(Error::Conflict(format!("role {id} is defined twice")));
        }
    }
    Ok(Arc::new(map))
}
