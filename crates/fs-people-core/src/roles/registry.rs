//! The role definitions loaded as configuration, one scheme per role file.
//!
//! Pattern: Registry, swapped atomically like a config plugin (ADR 0045, learned from Zentinel) —
//! a new file is checked completely first; readers see the old or the new set, never a mix. The
//! registry is keyed by base address, so our file and yours sit side by side and loading one
//! replaces only that one. It is the default `RoleDirectory`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

use super::definition::RoleDefinition;
use super::directory::RoleDirectory;
use super::ident::full_id;
use super::scheme::RoleScheme;
use crate::error::{Error, Result};

/// What this program ships. Data, not code.
const KNOWN: &str = include_str!("known.json");

type Schemes = Arc<BTreeMap<String, Arc<RoleScheme>>>;

/// Role definitions from configuration. Cheap to clone; clones share the same definitions.
#[derive(Clone, Debug)]
pub struct RoleRegistry(Arc<RwLock<Schemes>>);

impl RoleRegistry {
    /// The roles this program ships.
    ///
    /// # Panics
    /// Never in a released build: `known.json` is part of the crate and covered by tests.
    pub fn known() -> Self {
        let registry = Self::empty();
        registry.load(KNOWN).expect("known.json is valid");
        registry
    }

    /// An empty registry.
    pub fn empty() -> Self {
        Self(Arc::new(RwLock::new(Arc::default())))
    }

    /// Check `json` (a role file) without changing anything; answers what it defines.
    pub fn check(json: &str) -> Result<RoleScheme> {
        RoleScheme::parse(json)
    }

    fn current(&self) -> Schemes {
        self.0.read().expect("role registry lock").clone()
    }

    /// Read, change and write under one write lock, so parallel loads cannot lose each other.
    fn swap(&self, scheme: RoleScheme) -> Result<()> {
        let mut guard = self.0.write().expect("role registry lock");
        *guard = merged(&guard, scheme)?;
        Ok(())
    }
}

/// `current` with `scheme` in it: replacing the scheme of its base, or, for the old array form,
/// merging over it. A full id may exist once across all bases.
fn merged(current: &Schemes, mut scheme: RoleScheme) -> Result<Schemes> {
    let mut next = (**current).clone();
    if let (true, Some(old)) = (scheme.merge, next.get(&scheme.base)) {
        let mut roles: Vec<RoleDefinition> = old
            .roles
            .iter()
            .filter(|d| scheme.roles.iter().all(|n| n.id != d.id))
            .cloned()
            .collect();
        roles.append(&mut scheme.roles);
        scheme.roles = roles;
    }
    next.insert(scheme.base.clone(), Arc::new(scheme));
    let mut seen = BTreeSet::new();
    for def in next.values().flat_map(|s| s.roles.iter()) {
        if !seen.insert(&def.id) {
            return Err(Error::Conflict(format!("role {} is defined twice", def.id)));
        }
    }
    Ok(Arc::new(next))
}

fn flatten(schemes: &Schemes) -> BTreeMap<String, RoleDefinition> {
    let roles = schemes.values().flat_map(|s| s.roles.iter());
    roles.map(|d| (d.id.clone(), d.clone())).collect()
}

impl RoleDirectory for RoleRegistry {
    fn preview(&self, json: &str) -> Result<BTreeMap<String, RoleDefinition>> {
        Ok(flatten(&merged(&self.current(), RoleScheme::parse(json)?)?))
    }

    fn load(&self, json: &str) -> Result<()> {
        self.swap(RoleScheme::parse(json)?)
    }

    fn load_stored(&self, json: &str) -> Result<()> {
        self.swap(RoleScheme::parse_stored(json)?)
    }

    fn find(&self, id: &str) -> Option<RoleDefinition> {
        flatten(&self.current()).remove(&full_id(id))
    }

    fn all(&self) -> Vec<RoleDefinition> {
        flatten(&self.current()).into_values().collect()
    }

    fn bases(&self) -> Vec<String> {
        self.current().keys().cloned().collect()
    }
}
