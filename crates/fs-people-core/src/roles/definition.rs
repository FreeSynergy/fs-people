//! A role definition: what a role is, who owns it, how far it reaches.
//!
//! Pattern: Value object — plain serde data with a lossless `extra`, the same for shipped and
//! self-made roles (ADR 0052). Mirrors `RoleDefinition` of `people.capnp`.

use serde::{Deserialize, Serialize};

use crate::card::{Extra, Map};
use crate::error::{Error, Issue, Result};

/// Id of the role that marks an AI (ADR 0054). Its definition is configuration like any other.
pub const AI_ROLE: &str = "ai";

/// How far a role reaches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoleScope {
    /// Only for me.
    #[default]
    Personal,
    /// Within a group or organisation.
    Group,
    /// Published for everyone.
    Public,
}

/// One extra field of a person in this role. The role's owner holds its values, not `fs-people`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoleField {
    /// Name of the field.
    pub name: String,
    /// The standard it follows, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    /// Plain words, language tag -> text.
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub description: Map<String>,
}

/// What a role is.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RoleDefinition {
    /// Unique id of the role.
    pub id: String,
    /// Names, language tag -> text.
    pub names: Map<String>,
    /// Plain words, language tag -> text.
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub description: Map<String>,
    /// The standard it follows; none means a gap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    /// What is extra about a person in this role.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<RoleField>,
    /// The program responsible for it.
    pub owner: String,
    /// How far it reaches.
    pub scope: RoleScope,
    /// Cards with this role must have this JSContact `kind`.
    #[serde(rename = "cardKind", skip_serializing_if = "Option::is_none")]
    pub card_kind: Option<String>,
    /// Everything else, preserved verbatim.
    #[serde(flatten)]
    pub extra: Extra,
}

impl RoleDefinition {
    /// A role with one English name, owned by `owner`.
    pub fn new(id: &str, name: &str, owner: &str, scope: RoleScope) -> Self {
        Self {
            id: id.into(),
            names: Map::from([("en".to_string(), name.to_string())]),
            owner: owner.into(),
            scope,
            ..Default::default()
        }
    }

    /// `id`, a name and an owner are required.
    pub fn check(&self) -> Result<()> {
        let mut issues = vec![];
        let mut require = |ok: bool, path: &str, message: &str| {
            if !ok {
                issues.push(Issue {
                    path: path.into(),
                    message: message.into(),
                });
            }
        };
        require(!self.id.trim().is_empty(), "/id", "required");
        require(!self.names.is_empty(), "/names", "at least one name");
        require(!self.owner.trim().is_empty(), "/owner", "required");
        if issues.is_empty() {
            Ok(())
        } else {
            Err(Error::Invalid(issues))
        }
    }
}
