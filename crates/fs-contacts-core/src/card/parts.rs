//! The JSContact sub-objects (`Name`, `EmailAddress`, `Phone`, ...).
//!
//! Pattern: Template method (macro form) — `sub_object!` fixes the skeleton every sub-object
//! shares (`@type`, typed members, a flattened `extra` map for lossless round trips); each
//! object only lists its own members.

// The members mirror the RFC 9553 property names one to one; documenting each would only repeat
// the specification.
#![allow(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::Extra;

macro_rules! sub_object {
    ($(#[$m:meta])* $name:ident { $( $(#[$fm:meta])* $f:ident : $t:ty ),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct $name {
            #[serde(rename = "@type", default, skip_serializing_if = "Option::is_none")]
            pub type_: Option<String>,
            $( $(#[$fm])* pub $f: $t, )*
            #[serde(flatten)]
            pub extra: Extra,
        }
    };
}

sub_object!(
    /// One part of a name (`given`, `surname`, ...).
    NameComponent {
        kind: String,
        value: String,
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
    Organization {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        units: Vec<Value>,
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

sub_object!(AddressComponent {
    kind: String,
    value: String,
});

sub_object!(
    Address {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        components: Vec<AddressComponent>,
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
    }
);

sub_object!(
    Link {
        #[serde(default)]
        uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
    }
);

sub_object!(
    Note {
        #[serde(default)]
        note: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        created: Option<String>,
    }
);
