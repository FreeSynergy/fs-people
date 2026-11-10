//! The JSContact sub-objects of resources: calendars, keys, directories, links, media (RFC 9553 sections 2.4, 2.6).
//!
//! Pattern: Template method (macro form) — `sub_object!` (in `sub_object.rs`) fixes the skeleton
//! every sub-object shares; each object here only lists its own members.

// The members mirror the RFC 9553 property names one to one; documenting each would only repeat
// the specification.
#![allow(missing_docs)]

use std::collections::BTreeMap;

use super::sub_object::sub_object;

sub_object!(
    SchedulingAddress {
        #[serde(default)]
        uri: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);

sub_object!(
    Calendar {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(default)]
        uri: String,
        #[serde(default, rename = "mediaType", skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);

sub_object!(
    CryptoKey {
        #[serde(default)]
        uri: String,
        #[serde(default, rename = "mediaType", skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);

sub_object!(
    Directory {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(default)]
        uri: String,
        #[serde(default, rename = "mediaType", skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(default, rename = "listAs", skip_serializing_if = "Option::is_none")]
        list_as: Option<u32>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);

sub_object!(
    Link {
        #[serde(default)]
        uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(default, rename = "mediaType", skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);

sub_object!(
    Media {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(default)]
        uri: String,
        #[serde(default, rename = "mediaType", skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(default, rename = "blobId", skip_serializing_if = "Option::is_none")]
        blob_id: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        contexts: BTreeMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pref: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);
