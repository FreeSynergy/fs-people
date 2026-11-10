//! The [`Card`] data type: a typed JSContact card with lossless extras.
//!
//! Pattern: Newtype-style value object — plain serde data; behaviour lives in `build`,
//! `display` and `vendor` so this file stays a pure description of the wire form.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::about::{Anniversary, Note, PersonalInfo};
use super::parts::{
    Address, EmailAddress, LanguagePref, Name, Nickname, OnlineService, Organization, Phone,
    SpeakToAs, Title,
};
use super::resources::{Calendar, CryptoKey, Directory, Link, Media, SchedulingAddress};
use super::types::{Extra, Map};

fn is_empty_map<T>(m: &Map<T>) -> bool {
    m.is_empty()
}

/// A JSContact card (RFC 9553).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Card {
    /// JSON `@type`; must be `"Card"`.
    #[serde(rename = "@type", default)]
    pub type_: String,
    /// JSContact version; must be `"1.0"`.
    #[serde(default)]
    pub version: String,
    /// Globally unique id of the card (immutable once stored).
    #[serde(default)]
    pub uid: String,
    /// `individual` (default), `group`, `org`, `location`, `device`, `application` or
    /// `<domain>:<value>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Default language of the card (`language`, RFC 5646).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Product that made the card (`prodId`).
    #[serde(default, rename = "prodId", skip_serializing_if = "Option::is_none")]
    pub prod_id: Option<String>,
    /// Creation time (`UTCDateTime`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// Time of the last change (`UTCDateTime`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    /// The name of the entity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
    /// Nicknames by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub nicknames: Map<Nickname>,
    /// Organisations by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub organizations: Map<Organization>,
    /// How to address the person (`speakAs`): grammatical gender and pronouns.
    #[serde(default, rename = "speakAs", skip_serializing_if = "Option::is_none")]
    pub speak_as: Option<SpeakToAs>,
    /// Job titles by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub titles: Map<Title>,
    /// E-mail addresses by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub emails: Map<EmailAddress>,
    /// Accounts in online services by id.
    #[serde(
        default,
        rename = "onlineServices",
        skip_serializing_if = "is_empty_map"
    )]
    pub online_services: Map<OnlineService>,
    /// Phone numbers by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub phones: Map<Phone>,
    /// Language preferences by id.
    #[serde(
        default,
        rename = "preferredLanguages",
        skip_serializing_if = "is_empty_map"
    )]
    pub preferred_languages: Map<LanguagePref>,
    /// Calendars by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub calendars: Map<Calendar>,
    /// Scheduling addresses by id.
    #[serde(
        default,
        rename = "schedulingAddresses",
        skip_serializing_if = "is_empty_map"
    )]
    pub scheduling_addresses: Map<SchedulingAddress>,
    /// Postal addresses by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub addresses: Map<Address>,
    /// Public keys by id.
    #[serde(default, rename = "cryptoKeys", skip_serializing_if = "is_empty_map")]
    pub crypto_keys: Map<CryptoKey>,
    /// Directories (where the card lives) by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub directories: Map<Directory>,
    /// Links by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub links: Map<Link>,
    /// Photos, logos and sounds by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub media: Map<Media>,
    /// Translations: language tag -> `PatchObject` over this card (RFC 8984 section 1.4.9).
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub localizations: Map<Map<Value>>,
    /// Special dates by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub anniversaries: Map<Anniversary>,
    /// Expertise, hobbies and interests by id.
    #[serde(default, rename = "personalInfo", skip_serializing_if = "is_empty_map")]
    pub personal_info: Map<PersonalInfo>,
    /// Notes by id.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub notes: Map<Note>,
    /// Group members: member card `uid` -> `true` (only for `kind: group`).
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub members: Map<bool>,
    /// RFC 9553 `relatedTo` (kept verbatim; not used for same-person links).
    #[serde(default, rename = "relatedTo", skip_serializing_if = "is_empty_map")]
    pub related_to: Map<Value>,
    /// Keywords, `keyword -> true`.
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub keywords: Map<bool>,
    /// Everything else, including vendor properties `<domain>:<name>`.
    #[serde(flatten)]
    pub extra: Extra,
}
