//! The validation rules for a whole card.
//!
//! Pattern: Specification (composite) — header, name, sub-object and member rules each report
//! into one `Sink`; `validate` turns a non-empty result into `Error::Invalid`.

use super::datetime::is_utc_datetime;
use super::kind::{KINDS, is_valid_kind};
use super::sink::Sink;
use super::sub_objects;
use crate::card::Card;
use crate::error::{Error, Issue, Result};

/// All problems found; empty = valid.
pub fn issues(card: &Card) -> Vec<Issue> {
    let mut sink = Sink::default();
    header(&mut sink, card);
    name(&mut sink, card);
    sub_objects::check(&mut sink, card);
    super::resources::check(&mut sink, card);
    members(&mut sink, card);
    sink.0
}

/// Validate; `Err(Error::Invalid)` lists every problem.
pub fn validate(card: &Card) -> Result<()> {
    let i = issues(card);
    if i.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(i))
    }
}

fn header(sink: &mut Sink, card: &Card) {
    if card.type_ != "Card" {
        sink.bad("/@type", "must be \"Card\"");
    }
    if card.version != "1.0" {
        sink.bad("/version", "required, must be \"1.0\" (RFC 9553)");
    }
    if card.uid.trim().is_empty() {
        sink.bad("/uid", "required");
    }
    if let Some(k) = &card.kind
        && !is_valid_kind(k)
    {
        let msg = format!("unknown kind {k:?}; use {KINDS:?} or a vendor value <domain>:<value>");
        sink.bad("/kind", &msg);
    }
    for (name, val) in [("created", &card.created), ("updated", &card.updated)] {
        if val.as_deref().is_some_and(|t| !is_utc_datetime(t)) {
            sink.bad(
                &format!("/{name}"),
                "must be a UTC date-time like 2026-11-05T09:00:00Z",
            );
        }
    }
}

fn name(sink: &mut Sink, card: &Card) {
    let Some(n) = &card.name else { return };
    if n.type_.as_deref().is_some_and(|t| t != "Name") {
        sink.bad("/name/@type", "must be \"Name\"");
    }
    if n.full.as_deref().is_none_or(|f| f.trim().is_empty()) && n.components.is_empty() {
        sink.bad("/name", "needs `full` or `components`");
    }
    for (i, c) in n.components.iter().enumerate() {
        if c.kind.is_empty() || c.value.is_empty() {
            sink.bad(
                &format!("/name/components/{i}"),
                "`kind` and `value` are required",
            );
        }
    }
}

fn members(sink: &mut Sink, card: &Card) {
    if !card.members.is_empty() && !card.is_group() {
        sink.bad("/members", "only allowed when kind is \"group\"");
    }
    for (uid, flag) in &card.members {
        if uid.is_empty() {
            sink.bad("/members", "empty member uid");
        }
        if !*flag {
            sink.bad(
                &format!("/members/{uid}"),
                "must be true (RFC 9553 Boolean map)",
            );
        }
    }
}
