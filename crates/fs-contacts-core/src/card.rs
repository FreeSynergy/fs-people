//! Typed JSContact (RFC 9553) `Card`.
//!
//! Every object keeps unknown and vendor properties (`<domain>:<name>`, RFC 9553 section 1.8.1)
//! in a flattened `extra` map, so a round trip through this model loses nothing.
//! Properties we do not model explicitly (`calendars`, `cryptoKeys`, `personalInfo`, ...) also
//! end up in `extra` and are therefore preserved.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `Id[T]` map of JSContact.
pub type Map<T> = BTreeMap<String, T>;
/// Unknown / vendor properties, preserved verbatim.
pub type Extra = BTreeMap<String, Value>;

/// Vendor prefix (ADR 0028): independent of the instance domain.
pub const VENDOR: &str = "freesynergy.net:";
/// Vendor property carrying DPV categories of sensitive data (ADR 0029). Allowed on the card and on any nested object.
pub const SENSITIVE_KEY: &str = "freesynergy.net:sensitive";
/// Vendor property: actor IRI / DID the card describes (AS2 `Profile.describes`).
pub const DESCRIBES_KEY: &str = "freesynergy.net:describes";

fn is_empty_map<T>(m: &Map<T>) -> bool {
    m.is_empty()
}

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

sub_object!(
    AddressComponent {
        kind: String,
        value: String,
    }
);

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

/// A JSContact card.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Card {
    #[serde(rename = "@type", default)]
    pub type_: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub uid: String,
    /// `individual` (default), `group`, `org`, `location`, `device`, `application` or `<domain>:<value>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub nicknames: Map<Nickname>,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub organizations: Map<Organization>,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub titles: Map<Title>,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub emails: Map<EmailAddress>,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub phones: Map<Phone>,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub addresses: Map<Address>,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub links: Map<Link>,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub notes: Map<Note>,
    /// Group members: member card `uid` -> `true` (only for `kind: group`).
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub members: Map<bool>,
    #[serde(default, rename = "relatedTo", skip_serializing_if = "is_empty_map")]
    pub related_to: Map<Value>,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub keywords: Map<bool>,
    /// Everything else, including vendor properties `<domain>:<name>`.
    #[serde(flatten)]
    pub extra: Extra,
}

impl Card {
    /// A new valid card with a fresh `urn:uuid:` UUIDv7 `uid` and `name.full`.
    pub fn new(kind: &str, full_name: impl Into<String>) -> Self {
        let now = crate::now_utc();
        Card {
            type_: "Card".into(),
            version: "1.0".into(),
            uid: format!("urn:uuid:{}", fs_store::ObjectId::new()),
            kind: Some(kind.into()),
            created: Some(now.clone()),
            updated: Some(now),
            name: Some(Name {
                full: Some(full_name.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn individual(full_name: impl Into<String>) -> Self {
        Self::new("individual", full_name)
    }

    pub fn group(full_name: impl Into<String>) -> Self {
        Self::new("group", full_name)
    }

    /// `kind`, defaulting to `individual`.
    pub fn kind(&self) -> &str {
        self.kind.as_deref().unwrap_or("individual")
    }

    pub fn is_group(&self) -> bool {
        self.kind() == "group"
    }

    /// Add an e-mail address under the next free id; returns the id.
    pub fn add_email(&mut self, address: impl Into<String>) -> String {
        let id = next_id(&self.emails, "e");
        self.emails.insert(
            id.clone(),
            EmailAddress {
                type_: Some("EmailAddress".into()),
                address: address.into(),
                ..Default::default()
            },
        );
        id
    }

    pub fn add_phone(&mut self, number: impl Into<String>) -> String {
        let id = next_id(&self.phones, "p");
        self.phones.insert(
            id.clone(),
            Phone {
                type_: Some("Phone".into()),
                number: number.into(),
                ..Default::default()
            },
        );
        id
    }

    pub fn add_organization(&mut self, name: impl Into<String>) -> String {
        let id = next_id(&self.organizations, "o");
        self.organizations.insert(
            id.clone(),
            Organization {
                type_: Some("Organization".into()),
                name: Some(name.into()),
                ..Default::default()
            },
        );
        id
    }

    /// Name for display and for the envelope `name` (ADR 0032): `name.full`, else the joined
    /// components, else nickname, organisation, e-mail, finally the `uid`. Never empty.
    pub fn display_name(&self) -> String {
        let non_empty = |s: &str| -> Option<String> {
            let t = s.trim();
            (!t.is_empty()).then(|| t.to_string())
        };
        if let Some(n) = &self.name {
            if let Some(f) = n.full.as_deref().and_then(non_empty) {
                return f;
            }
            let parts: Vec<&str> = n
                .components
                .iter()
                .filter(|c| !c.value.trim().is_empty() && c.kind != "separator")
                .map(|c| c.value.trim())
                .collect();
            if !parts.is_empty() {
                return parts.join(" ");
            }
        }
        self.nicknames
            .values()
            .find_map(|n| non_empty(&n.name))
            .or_else(|| self.organizations.values().find_map(|o| o.name.as_deref().and_then(non_empty)))
            .or_else(|| self.emails.values().find_map(|e| non_empty(&e.address)))
            .or_else(|| non_empty(&self.uid))
            .unwrap_or_else(|| "Unnamed contact".into())
    }

    /// Vendor / unknown property.
    pub fn vendor(&self, key: &str) -> Option<&Value> {
        self.extra.get(key)
    }

    pub fn set_vendor(&mut self, key: &str, value: Value) {
        self.extra.insert(key.into(), value);
    }

    /// Actor IRI / DID this card describes (AS2 `Profile.describes`).
    pub fn describes(&self) -> Option<&str> {
        self.vendor(DESCRIBES_KEY).and_then(Value::as_str)
    }

    /// Mark the whole card as holding sensitive data of the given DPV categories (ADR 0029).
    pub fn mark_sensitive<I, S>(&mut self, categories: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let list: Vec<Value> = categories.into_iter().map(|c| Value::String(c.into())).collect();
        self.extra.insert(SENSITIVE_KEY.into(), Value::Array(list));
    }

    /// All DPV categories marked anywhere in the card (card level or any nested object), sorted, unique.
    pub fn sensitive_categories(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Ok(v) = serde_json::to_value(self) {
            collect_sensitive(&v, &mut out);
        }
        out.sort();
        out.dedup();
        out
    }

    pub fn is_sensitive(&self) -> bool {
        !self.sensitive_categories().is_empty()
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("Card serialises")
    }
}

fn next_id<T>(m: &Map<T>, prefix: &str) -> String {
    (1..).map(|i| format!("{prefix}{i}")).find(|k| !m.contains_key(k)).unwrap()
}

fn collect_sensitive(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(o) => {
            for (k, v) in o {
                if k == SENSITIVE_KEY {
                    match v {
                        Value::Array(a) => out.extend(a.iter().filter_map(|x| x.as_str().map(str::to_string))),
                        Value::String(s) => out.push(s.clone()),
                        _ => {}
                    }
                } else {
                    collect_sensitive(v, out);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| collect_sensitive(x, out)),
        _ => {}
    }
}
