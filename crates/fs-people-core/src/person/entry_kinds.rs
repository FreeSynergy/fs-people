//! The kind a new entry gets: which kinds are allowed, and the one that is preselected.
//!
//! Pattern: Strategy table — one row per multiple field whose entries have a `kind` in RFC 9553. The
//! description reports each row as a choice field (`mediaKind`: `photo`, `logo`, `sound`), so a form
//! asks for the kind with a selection that the program, not the frontend, defines. A choice key is lower
//! case words (`free-busy`); the RFC value (`freeBusy`) is in the parallel list.

use std::collections::BTreeMap;

/// One row.
pub(crate) struct EntryKind {
    /// The field of the entries (`media`).
    pub field: &'static str,
    /// The name of the selector field in the description (`mediaKind`).
    pub selector: &'static str,
    /// The allowed keys of the selector, in the order a form shows them. The first is the preselected one (the description
    /// has no member for a default; this order is the convention, test `the_first_kind_is_the_default`).
    pub keys: &'static [&'static str],
    /// The RFC 9553 value of each key.
    pub rfc: &'static [&'static str],
    /// The key that is preselected (provisional, decision 8 of the strategy for the first four).
    pub default: &'static str,
}

pub(crate) const ENTRY_KINDS: [EntryKind; 5] = [
    EntryKind {
        field: "calendars",
        selector: "calendarKind",
        keys: &["calendar", "free-busy"],
        rfc: &["calendar", "freeBusy"],
        default: "calendar",
    },
    EntryKind {
        field: "directories",
        selector: "directoryKind",
        keys: &["directory", "entry"],
        rfc: &["directory", "entry"],
        default: "directory",
    },
    EntryKind {
        field: "media",
        selector: "mediaKind",
        // No `sound`: `media` is an image field (basic type `image`), which only knows pictures. A sound would be a field
        // of its own with a type for sound, which `types.capnp` has not (named in DESIGN.md).
        keys: &["photo", "logo"],
        rfc: &["photo", "logo"],
        default: "photo",
    },
    EntryKind {
        field: "personalInfo",
        selector: "personalInfoKind",
        keys: &["interest", "expertise", "hobby"],
        rfc: &["interest", "expertise", "hobby"],
        default: "interest",
    },
    // Not among the first four of the decision; the same rule, the default is provisional.
    EntryKind {
        field: "anniversaries",
        selector: "anniversaryKind",
        keys: &["birth", "death", "wedding"],
        rfc: &["birth", "death", "wedding"],
        default: "birth",
    },
];

/// The kinds an edit chose for the entries it creates: selector name to key.
#[derive(Default)]
pub(crate) struct Chosen(BTreeMap<String, String>);

impl Chosen {
    pub(crate) fn set(&mut self, selector: &str, key: &str) {
        self.0.insert(selector.to_owned(), key.to_owned());
    }

    /// The RFC kind for a new entry of `field`: the one chosen, else the preselected one.
    pub(crate) fn kind(&self, field: &str) -> String {
        let Some(row) = ENTRY_KINDS.iter().find(|r| r.field == field) else {
            return String::new();
        };
        let key = self.0.get(row.selector).map_or(row.default, String::as_str);
        let at = row.keys.iter().position(|k| *k == key).unwrap_or(0);
        row.rfc[at].to_owned()
    }
}

/// Whether `name` is a selector field.
pub(crate) fn is_selector(name: &str) -> bool {
    ENTRY_KINDS.iter().any(|r| r.selector == name)
}
