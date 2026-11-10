//! The `sub_object!` macro: the skeleton every JSContact sub-object shares.
//!
//! Pattern: Template method (macro form) — `@type`, typed members and a flattened `extra` map for
//! lossless round trips are fixed here; each object only lists its own members.

macro_rules! sub_object {
    ($(#[$m:meta])* $name:ident { $( $(#[$fm:meta])* $f:ident : $t:ty ),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(default)]
        pub struct $name {
            #[serde(rename = "@type", default, skip_serializing_if = "Option::is_none")]
            pub type_: Option<String>,
            $( $(#[$fm])* pub $f: $t, )*
            #[serde(flatten)]
            pub extra: $crate::card::Extra,
        }
    };
}

pub(super) use sub_object;
