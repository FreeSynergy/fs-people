//! The JSContact sub-objects of additional information: anniversaries, notes, personal information (RFC 9553 section 2.8).
//!
//! Pattern: Template method (macro form) — `sub_object!` (in `sub_object.rs`) fixes the skeleton
//! every sub-object shares; each object here only lists its own members.

// The members mirror the RFC 9553 property names one to one; documenting each would only repeat
// the specification.
#![allow(missing_docs)]

use super::parts::Address;
use super::sub_object::sub_object;

sub_object!(
    /// A `PartialDate` (year, month, day, calendar scale) or a `Timestamp` (`utc`); the `@type` tells which.
    AnniversaryDate {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        year: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        month: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        day: Option<u32>,
        #[serde(default, rename = "calendarScale", skip_serializing_if = "Option::is_none")]
        calendar_scale: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        utc: Option<String>,
    }
);

sub_object!(
    Anniversary {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        date: Option<AnniversaryDate>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        place: Option<Address>,
    }
);

sub_object!(
    Author {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        uri: Option<String>,
    }
);

sub_object!(
    Note {
        #[serde(default)]
        note: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        created: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<Author>,
    }
);

sub_object!(
    PersonalInfo {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        #[serde(default)]
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        level: Option<String>,
        #[serde(default, rename = "listAs", skip_serializing_if = "Option::is_none")]
        list_as: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    }
);
