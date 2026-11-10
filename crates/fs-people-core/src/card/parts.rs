//! The JSContact sub-objects of names, organisations and contact points (RFC 9553 sections 2.2, 2.3, 2.5).
//!
//! Pattern: Template method (macro form) — `sub_object!` (in `sub_object.rs`) fixes the skeleton
//! every sub-object shares; each object here only lists its own members.

// The members mirror the RFC 9553 property names one to one; documenting each would only repeat
// the specification.
#![allow(missing_docs)]

use std::collections::BTreeMap;

use super::sub_object::sub_object;

/// `Name.sortAs`: RFC 9553 section 2.2.1.1 makes it a map from component kind to sort string
/// (`{"surname": "Lovelace"}`). Cards written before carry a single string; both are read and written
/// back as they came, so no stored card becomes unreadable.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum SortAs {
    /// Component kind to sort string (the RFC form).
    ByKind(BTreeMap<String, String>),
    /// One string for the whole name (older cards).
    Whole(String),
}

sub_object!(
    /// One part of a name (`given`, `surname`, ...).
    NameComponent {
        #[serde(default)]
        kind: String,
        #[serde(default)]
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phonetic: Option<String>,
    }
);

sub_object!(
    Name {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        components: Vec<NameComponent>,
        #[serde(default, rename = "isOrdered", skip_serializing_if = "Option::is_none")]
        is_ordered: Option<bool>,
        #[serde(default, rename = "defaultSeparator", skip_serializing_if = "Option::is_none")]
        default_separator: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full: Option<String>,
        #[serde(default, rename = "sortAs", skip_serializing_if = "Option::is_none")]
        sort_as: Option<SortAs>,
        #[serde(default, rename = "phoneticScript", skip_serializing_if = "Option::is_none")]
        phonetic_script: Option<String>,
        #[serde(default, rename = "phoneticSystem", skip_serializing_if = "Option::is_none")]
        phonetic_system: Option<String>,
    }
);

sub_object!(
    Nickname {
        #[serde(default)]
        name: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
    }
);

sub_object!(
    OrgUnit {
        #[serde(default)]
        name: String,
        #[serde(default, rename = "sortAs", skip_serializing_if = "Option::is_none")]
        sort_as: Option<String>,
    }
);

sub_object!(
    Organization {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        units: Vec<OrgUnit>,
        #[serde(default, rename = "sortAs", skip_serializing_if = "Option::is_none")]
        sort_as: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
    }
);

sub_object!(
    Title {
        #[serde(default)]
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(default, rename = "organizationId", skip_serializing_if = "Option::is_none")]
        organization_id: Option<String>,
    }
);

sub_object!(
    Pronouns {
        #[serde(default)]
        pronouns: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
    }
);

sub_object!(
    SpeakToAs {
        #[serde(default, rename = "grammaticalGender", skip_serializing_if = "Option::is_none")]
        grammatical_gender: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        pronouns: BTreeMap<String, Pronouns>,
    }
);

sub_object!(
    EmailAddress {
        #[serde(default)]
        address: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);

sub_object!(
    Phone {
        #[serde(default)]
        number: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        features: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);

sub_object!(
    OnlineService {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        service: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uri: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        user: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);

sub_object!(
    LanguagePref {
        #[serde(default)]
        language: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
    }
);

sub_object!(
    AddressComponent {
        #[serde(default)]
        kind: String,
        #[serde(default)]
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        phonetic: Option<String>,
    }
);

sub_object!(
    Address {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        components: Vec<AddressComponent>,
        #[serde(default, rename = "isOrdered", skip_serializing_if = "Option::is_none")]
        is_ordered: Option<bool>,
        #[serde(default, rename = "defaultSeparator", skip_serializing_if = "Option::is_none")]
        default_separator: Option<String>,
        #[serde(default, rename = "countryCode", skip_serializing_if = "Option::is_none")]
        country_code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        coordinates: Option<String>,
        #[serde(default, rename = "timeZone", skip_serializing_if = "Option::is_none")]
        time_zone: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        full: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
    }
);
