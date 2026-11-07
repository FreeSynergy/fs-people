//! Calls of `Persons.list`.
//!
//! Pattern: Adapter — one request in, one page read back as values of the core; `list` walks the
//! pages until the cursor is empty, or until enough is there for the `limit` asked for.

use fs_interfaces::people_capnp::persons::Client;
use fs_people_core::{ListFilter, Result};
use fs_store::StoredObject;

use crate::pages::{Cursor, PersonPage};
use crate::problems::{Subject, error_of};
use crate::read::{stored, unreadable};

/// One page of `list`: the page at `cursor` (none = the first).
pub(crate) async fn page(
    c: &Client,
    filter: &ListFilter,
    cursor: Option<&Cursor>,
) -> Result<PersonPage> {
    let mut req = c.list_request();
    let mut p = req.get();
    let mut wire = p.reborrow().init_filter();
    wire.set_kind(filter.kind.as_deref().unwrap_or(""));
    wire.set_role(filter.role.as_deref().unwrap_or(""));
    wire.set_term(filter.term.as_deref().unwrap_or(""));
    p.init_cursor()
        .set_opaque(cursor.map_or(&[][..], Cursor::as_bytes));
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::None))?;
    let page = answer
        .get()
        .and_then(|r| r.get_page())
        .map_err(unreadable)?;
    let items = page
        .get_items()
        .map_err(unreadable)?
        .iter()
        .map(stored)
        .collect::<Result<Vec<_>>>()?;
    let next = page
        .get_next()
        .and_then(|n| n.get_opaque())
        .map_err(unreadable)?
        .to_vec();
    Ok(PersonPage {
        items,
        next: (!next.is_empty()).then(|| Cursor::from_bytes(next)),
    })
}

/// `list`, every page needed. `limit` and `offset` of the filter are applied here: the contract has
/// none, so the pages before `offset` are read as well (`PersonPages` reads on from a cursor).
pub(crate) async fn list(c: &Client, filter: &ListFilter) -> Result<Vec<StoredObject>> {
    let enough = filter
        .limit
        .map(|limit| filter.offset.saturating_add(limit));
    let mut found: Vec<StoredObject> = Vec::new();
    let mut cursor: Option<Cursor> = None;
    loop {
        let got = page(c, filter, cursor.as_ref()).await?;
        let empty = got.items.is_empty();
        found.extend(got.items);
        cursor = got.next;
        if empty || cursor.is_none() || enough.is_some_and(|n| found.len() >= n) {
            break;
        }
    }
    let rest = found.into_iter().skip(filter.offset);
    Ok(match filter.limit {
        Some(limit) => rest.take(limit).collect(),
        None => rest.collect(),
    })
}
