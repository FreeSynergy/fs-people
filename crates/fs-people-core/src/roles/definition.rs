//! A role definition: what a role is, who owns it, how far it reaches.
//!
//! Pattern: Value object — plain serde data with a lossless `extra`, the same for shipped and
//! self-made roles (ADR 0052). It mirrors one entry of a role file (a SKOS concept in JSON-LD) and
//! `RoleDefinition` of `people.capnp`.

use serde::{Deserialize, Serialize};

use super::ident::full_id;
use crate::card::{Extra, Map};
use crate::error::{Error, Issue, Result};

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

/// What a role is in SKOS terms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleKind {
    /// A role inside an organisation (W3C `org`, section 5.3.5).
    #[serde(rename = "org:Role")]
    OrgRole,
    /// A described concept that is no role inside an organisation, such as customer or supplier.
    #[default]
    #[serde(rename = "skos:Concept")]
    Concept,
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
    /// Full id of the role: the base address of its file plus its short id. A role file only
    /// writes the short id.
    pub id: String,
    /// `org:Role` or `skos:Concept`.
    #[serde(rename = "type")]
    pub kind: RoleKind,
    /// Names, language tag -> text.
    pub names: Map<String>,
    /// Plain words, language tag -> text.
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub description: Map<String>,
    /// The more general role, also in another file (full id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broader: Option<String>,
    /// The same idea in another file (full id). Provisional: one address.
    #[serde(rename = "closeMatch", skip_serializing_if = "Option::is_none")]
    pub close_match: Option<String>,
    /// The standard it follows; none means a gap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    /// Addresses of the standard's terms.
    #[serde(rename = "standardRefs", skip_serializing_if = "Vec::is_empty")]
    pub standard_refs: Vec<String>,
    /// What is extra about a person in this role.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<RoleField>,
    /// The program responsible for it, as an id.
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
    /// A role with one English name, owned by `owner`. A short `id` is one of our roles.
    pub fn new(id: &str, name: &str, owner: &str, scope: RoleScope) -> Self {
        Self {
            id: full_id(id),
            names: Map::from([("en".to_string(), name.to_string())]),
            owner: owner.into(),
            scope,
            ..Default::default()
        }
    }

    /// `id`, a name and an owner (an id, not a sentence) are required.
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
        let named = !self.id.trim().is_empty() && !self.id.ends_with(['/', '#']);
        require(named, "/id", "required, and more than a base address");
        require(!self.names.is_empty(), "/names", "at least one name");
        require(
            is_id(&self.owner),
            "/owner",
            "an id such as fs-people, no blanks",
        );
        let kind_named = self.card_kind.as_ref().is_none_or(|k| !k.trim().is_empty());
        require(kind_named, "/cardKind", "leave it out, or name a kind");
        if issues.is_empty() {
            Ok(())
        } else {
            Err(Error::Invalid(issues))
        }
    }
}

/// An owner from before owners were ids ("CRM (later)") as an id ("crm-later"). Only for data that
/// was accepted once; new input is checked as it is.
pub(super) fn owner_id(owner: &str) -> String {
    if is_id(owner) {
        return owner.to_string();
    }
    let words: Vec<String> = owner
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    // Nothing readable left ("?"): a marked gap is better than losing the whole file, and it says
    // what it is. It is not `fs-people`, so no details are kept for the role (the safe side).
    if words.is_empty() {
        return "unknown".into();
    }
    words.join("-")
}

/// An owner is an id: no blanks, no brackets.
fn is_id(owner: &str) -> bool {
    !owner.is_empty()
        && owner
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/' | '#'))
}
