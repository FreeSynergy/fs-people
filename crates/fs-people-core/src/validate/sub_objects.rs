//! Rules for the keyed sub-objects of a card (e-mails, phones, links, ...).
//!
//! Pattern: Specification — one table-like call per property: the type name and the
//! "what makes it unusable" check; the shared checks (empty id, wrong `@type`) live in one place.

use super::sink::Sink;
use crate::card::{Address, EmailAddress, Link, Nickname, Note, Organization, Phone, Title};
use crate::card::{Card, Map};

/// Check one `Id[T]` property: empty ids, `@type` and a property specific problem.
fn each<T>(
    sink: &mut Sink,
    field: &str,
    ty: &str,
    items: &Map<T>,
    type_of: impl Fn(&T) -> Option<&str>,
    problem: impl Fn(&T) -> Option<&'static str>,
) {
    for (id, o) in items {
        let path = format!("/{field}/{id}");
        if id.is_empty() {
            sink.bad(&path, "empty Id");
        }
        if type_of(o).is_some_and(|t| t != ty) {
            sink.bad(&format!("{path}/@type"), &format!("must be \"{ty}\""));
        }
        if let Some(msg) = problem(o) {
            sink.bad(&path, msg);
        }
    }
}

fn blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// Run the rules for all sub-object properties of `card`.
pub(super) fn check(sink: &mut Sink, card: &Card) {
    let req = |cond: bool, msg: &'static str| cond.then_some(msg);
    each(
        sink,
        "emails",
        "EmailAddress",
        &card.emails,
        |o: &EmailAddress| o.type_.as_deref(),
        |o| req(blank(&o.address), "`address` is required"),
    );
    each(
        sink,
        "phones",
        "Phone",
        &card.phones,
        |o: &Phone| o.type_.as_deref(),
        |o| req(blank(&o.number), "`number` is required"),
    );
    each(
        sink,
        "links",
        "Link",
        &card.links,
        |o: &Link| o.type_.as_deref(),
        |o| req(blank(&o.uri), "`uri` is required"),
    );
    each(
        sink,
        "notes",
        "Note",
        &card.notes,
        |o: &Note| o.type_.as_deref(),
        |o| req(o.note.is_empty(), "`note` is required"),
    );
    each(
        sink,
        "nicknames",
        "Nickname",
        &card.nicknames,
        |o: &Nickname| o.type_.as_deref(),
        |o| req(blank(&o.name), "`name` is required"),
    );
    each(
        sink,
        "titles",
        "Title",
        &card.titles,
        |o: &Title| o.type_.as_deref(),
        |o| req(blank(&o.name), "`name` is required"),
    );
    each(
        sink,
        "organizations",
        "Organization",
        &card.organizations,
        |o: &Organization| o.type_.as_deref(),
        |o| {
            let no_name = o.name.as_deref().is_none_or(blank);
            req(no_name && o.units.is_empty(), "needs `name` or `units`")
        },
    );
    each(
        sink,
        "addresses",
        "Address",
        &card.addresses,
        |o: &Address| o.type_.as_deref(),
        |o| {
            let empty = o.components.is_empty()
                && o.full.is_none()
                && o.coordinates.is_none()
                && o.country_code.is_none();
            req(empty, "needs components, full, coordinates or countryCode")
        },
    );
}
