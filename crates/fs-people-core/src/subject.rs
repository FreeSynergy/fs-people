//! Whom an error is about: the kind of object and its id, as plain facts.
//!
//! Pattern: Value object, with the one function that reads it off an error — `Error::subject` only looks at
//! the variant (no state, no side effects), so a server can name the object in a typed field instead of
//! a sentence (`coding.md` §2a).

use std::fmt;

use crate::error::Error;

/// The sort of object an error is about, the contract's own noun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A card of a person (or anything else that is not a list).
    Person,
    /// A list: a card of kind group.
    List,
    /// A role definition.
    Role,
}

impl Kind {
    /// The noun on the wire: `person`, `list`, `role`.
    pub fn noun(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::List => "list",
            Self::Role => "role",
        }
    }
}

/// An object, by kind and id, that an error is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    /// What sort of object.
    pub kind: Kind,
    /// Its id as text: an object id, or the full id of a role.
    pub id: String,
}

impl Subject {
    /// A subject of this kind.
    pub fn new(kind: Kind, id: impl fmt::Display) -> Self {
        Self {
            kind,
            id: id.to_string(),
        }
    }
}

impl Error {
    /// The object this error is about, if the error itself knows it. A conflict or an invalid card
    /// is not tied to one object in the error, so it has none.
    pub fn subject(&self) -> Option<Subject> {
        match self {
            Self::NotFound(id)
            | Self::Changed(id)
            | Self::Sensitive(id)
            | Self::PartlyDeleted(id, _) => Some(Subject::new(Kind::Person, id)),
            Self::ListNotFound(id) => Some(Subject::new(Kind::List, id)),
            Self::UnknownRole(id) => Some(Subject::new(Kind::Role, id)),
            Self::NotReleased(subject, _) | Self::SensitiveMembers(subject, _) => {
                Some(subject.clone())
            }
            _ => None,
        }
    }
}
