//! Validation of a card against the required parts of RFC 9553.

use crate::card::Card;
use crate::error::{Error, Issue, Result};

/// `kind` values of RFC 9553 section 2.1.4.
pub const KINDS: [&str; 6] = ["individual", "group", "org", "location", "device", "application"];

/// `kind` is one of the registered values or a vendor value `<domain>:<value>`.
pub fn is_valid_kind(k: &str) -> bool {
    KINDS.contains(&k) || matches!(k.split_once(':'), Some((d, v)) if !d.is_empty() && !v.is_empty() && d.contains('.'))
}

/// `YYYY-MM-DDTHH:MM:SS[.fff]Z` (RFC 9553 `UTCDateTime`).
pub fn is_utc_datetime(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20 || !s.ends_with('Z') {
        return false;
    }
    let digit = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let fixed = digit(0..4)
        && b[4] == b'-'
        && digit(5..7)
        && b[7] == b'-'
        && digit(8..10)
        && b[10] == b'T'
        && digit(11..13)
        && b[13] == b':'
        && digit(14..16)
        && b[16] == b':'
        && digit(17..19);
    if !fixed {
        return false;
    }
    let rest = &s[19..s.len() - 1];
    rest.is_empty() || (rest.starts_with('.') && rest.len() > 1 && rest[1..].bytes().all(|c| c.is_ascii_digit()))
}

/// All problems found; empty = valid.
pub fn issues(card: &Card) -> Vec<Issue> {
    let mut v = Vec::new();
    let mut bad = |path: &str, msg: &str| v.push(Issue { path: path.into(), message: msg.into() });

    if card.type_ != "Card" {
        bad("/@type", "must be \"Card\"");
    }
    if card.version != "1.0" {
        bad("/version", "required, must be \"1.0\" (RFC 9553)");
    }
    if card.uid.trim().is_empty() {
        bad("/uid", "required");
    }
    if let Some(k) = &card.kind {
        if !is_valid_kind(k) {
            bad("/kind", &format!("unknown kind {k:?}; use {KINDS:?} or a vendor value <domain>:<value>"));
        }
    }
    for (name, val) in [("created", &card.created), ("updated", &card.updated)] {
        if let Some(t) = val {
            if !is_utc_datetime(t) {
                bad(&format!("/{name}"), "must be a UTC date-time like 2026-11-05T09:00:00Z");
            }
        }
    }
    if let Some(n) = &card.name {
        if n.type_.as_deref().is_some_and(|t| t != "Name") {
            bad("/name/@type", "must be \"Name\"");
        }
        if n.full.as_deref().is_none_or(|f| f.trim().is_empty()) && n.components.is_empty() {
            bad("/name", "needs `full` or `components`");
        }
        for (i, c) in n.components.iter().enumerate() {
            if c.kind.is_empty() || c.value.is_empty() {
                bad(&format!("/name/components/{i}"), "`kind` and `value` are required");
            }
        }
    }
    macro_rules! each {
        ($field:ident, $ty:literal, $check:expr) => {
            for (id, o) in &card.$field {
                let path = format!("/{}/{}", stringify!($field), id);
                if id.is_empty() {
                    bad(&path, "empty Id");
                }
                if o.type_.as_deref().is_some_and(|t| t != $ty) {
                    bad(&format!("{path}/@type"), concat!("must be \"", $ty, "\""));
                }
                #[allow(clippy::redundant_closure_call)]
                if let Some(msg) = ($check)(o) {
                    bad(&path, msg);
                }
            }
        };
    }
    each!(emails, "EmailAddress", |o: &crate::card::EmailAddress| o.address.trim().is_empty().then_some("`address` is required"));
    each!(phones, "Phone", |o: &crate::card::Phone| o.number.trim().is_empty().then_some("`number` is required"));
    each!(links, "Link", |o: &crate::card::Link| o.uri.trim().is_empty().then_some("`uri` is required"));
    each!(notes, "Note", |o: &crate::card::Note| o.note.is_empty().then_some("`note` is required"));
    each!(nicknames, "Nickname", |o: &crate::card::Nickname| o.name.trim().is_empty().then_some("`name` is required"));
    each!(titles, "Title", |o: &crate::card::Title| o.name.trim().is_empty().then_some("`name` is required"));
    each!(organizations, "Organization", |o: &crate::card::Organization| {
        (o.name.as_deref().is_none_or(|n| n.trim().is_empty()) && o.units.is_empty()).then_some("needs `name` or `units`")
    });
    each!(addresses, "Address", |o: &crate::card::Address| {
        (o.components.is_empty() && o.full.is_none() && o.coordinates.is_none() && o.country_code.is_none())
            .then_some("needs components, full, coordinates or countryCode")
    });

    if !card.members.is_empty() && !card.is_group() {
        bad("/members", "only allowed when kind is \"group\"");
    }
    for (uid, flag) in &card.members {
        if uid.is_empty() {
            bad("/members", "empty member uid");
        }
        if !*flag {
            bad(&format!("/members/{uid}"), "must be true (RFC 9553 Boolean map)");
        }
    }
    v
}

/// Validate; `Err(Error::Invalid)` lists every problem.
pub fn validate(card: &Card) -> Result<()> {
    let i = issues(card);
    if i.is_empty() { Ok(()) } else { Err(Error::Invalid(i)) }
}
