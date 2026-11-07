//! The role definitions loaded as configuration, one scheme per role file.
//!
//! Pattern: Registry, swapped atomically like a config plugin (ADR 0045, learned from Zentinel) —
//! a new file is checked completely first; readers see the old or the new set, never a mix. The
//! registry is keyed by base address, so our file and yours sit side by side and loading one
//! replaces only that one.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use super::definition::RoleDefinition;
use super::directory::RoleDirectory;
use super::ident::full_id;
use super::scheme::RoleScheme;
use crate::error::Result;

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

    fn with(&self, scheme: RoleScheme) -> Schemes {
        let mut next = (*self.current()).clone();
        next.insert(scheme.base.clone(), Arc::new(scheme));
        Arc::new(next)
    }
}

fn flatten(schemes: &Schemes) -> BTreeMap<String, RoleDefinition> {
    let roles = schemes.values().flat_map(|s| s.roles.iter());
    roles.map(|d| (d.id.clone(), d.clone())).collect()
}

impl RoleDirectory for RoleRegistry {
    fn preview(&self, json: &str) -> Result<BTreeMap<String, RoleDefinition>> {
        Ok(flatten(&self.with(RoleScheme::parse(json)?)))
    }

    /// # Panics
    /// Only if another thread panicked while swapping, which holds no user code.
    fn load(&self, json: &str) -> Result<()> {
        let next = self.with(RoleScheme::parse(json)?);
        *self.0.write().expect("role registry lock") = next;
        Ok(())
    }

    fn find(&self, id: &str) -> Option<RoleDefinition> {
        let full = full_id(id);
        self.current()
            .values()
            .find_map(|s| s.roles.iter().find(|d| d.id == full).cloned())
    }

    fn all(&self) -> Vec<RoleDefinition> {
        flatten(&self.current()).into_values().collect()
    }

    fn bases(&self) -> Vec<String> {
        self.current().keys().cloned().collect()
    }
}
