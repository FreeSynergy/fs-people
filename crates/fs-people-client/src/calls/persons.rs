//! Calls of `Persons`.
//!
//! Pattern: Adapter — one request in, one reply read back as values of the core. A list is read
//! page by page until the cursor is empty, or until enough is there for the `limit` asked for.

use fs_interfaces::people_capnp::persons::Client;
use fs_interfaces::rpc::write_object_id;
use fs_people_core::{Card, ImportReport, ListFilter, ReleasedExport, Result};
use fs_store::{ObjectId, StoredObject};

use crate::problems::{Subject, error_of};
use crate::read::{report, stored, unreadable};

fn person(id: ObjectId) -> Subject {
    Subject::Person(id)
}

/// `create`: the card as JSContact text.
pub(crate) async fn create(c: &Client, card: String) -> Result<StoredObject> {
    let mut req = c.create_request();
    req.get().set_jscontact(card.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::None))?;
    stored(
        answer
            .get()
            .and_then(|r| r.get_person())
            .map_err(unreadable)?,
    )
}

/// `update`.
pub(crate) async fn update(c: &Client, id: ObjectId, card: String) -> Result<StoredObject> {
    let mut req = c.update_request();
    write_object_id(req.get().init_id(), id);
    req.get().set_jscontact(card.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &person(id)))?;
    stored(
        answer
            .get()
            .and_then(|r| r.get_person())
            .map_err(unreadable)?,
    )
}

/// `delete`.
pub(crate) async fn delete(c: &Client, id: ObjectId) -> Result<()> {
    let mut req = c.delete_request();
    write_object_id(req.get().init_id(), id);
    req.send()
        .promise
        .await
        .map_err(|e| error_of(&e, &person(id)))?;
    Ok(())
}

/// `get`: `None` when the server says `found = false`.
pub(crate) async fn get(c: &Client, id: ObjectId) -> Result<Option<StoredObject>> {
    let mut req = c.get_request();
    write_object_id(req.get().init_id(), id);
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &person(id)))?;
    let r = answer.get().map_err(unreadable)?;
    if !r.get_found() {
        return Ok(None);
    }
    stored(r.get_person().map_err(unreadable)?).map(Some)
}

/// `list`, every page needed. `limit` and `offset` of the filter are applied here: the contract has
/// none, so the pages before `offset` are read as well.
pub(crate) async fn list(c: &Client, filter: &ListFilter) -> Result<Vec<StoredObject>> {
    let enough = filter
        .limit
        .map(|limit| filter.offset.saturating_add(limit));
    let mut found: Vec<StoredObject> = Vec::new();
    let mut cursor: Vec<u8> = Vec::new();
    loop {
        let mut req = c.list_request();
        let mut p = req.get();
        let mut wire = p.reborrow().init_filter();
        wire.set_kind(filter.kind.as_deref().unwrap_or(""));
        wire.set_role(filter.role.as_deref().unwrap_or(""));
        wire.set_term(filter.term.as_deref().unwrap_or(""));
        p.init_cursor().set_opaque(&cursor);
        let answer = req
            .send()
            .promise
            .await
            .map_err(|e| error_of(&e, &Subject::None))?;
        let page = answer
            .get()
            .and_then(|r| r.get_page())
            .map_err(unreadable)?;
        let items = page.get_items().map_err(unreadable)?;
        let before = found.len();
        for item in items {
            found.push(stored(item)?);
        }
        cursor = page
            .get_next()
            .and_then(|n| n.get_opaque())
            .map_err(unreadable)?
            .to_vec();
        let done = cursor.is_empty() || found.len() == before;
        if done || enough.is_some_and(|n| found.len() >= n) {
            break;
        }
    }
    let rest = found.into_iter().skip(filter.offset);
    Ok(match filter.limit {
        Some(limit) => rest.take(limit).collect(),
        None => rest.collect(),
    })
}

/// `importText`.
pub(crate) async fn import(
    c: &Client,
    media_type: &'static str,
    text: String,
) -> Result<ImportReport> {
    let mut req = c.import_text_request();
    let mut p = req.get();
    p.set_media_type(media_type);
    p.set_text(text.as_str());
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &Subject::None))?;
    report(
        answer
            .get()
            .and_then(|r| r.get_report())
            .map_err(unreadable)?,
    )
}

/// `exportText`.
pub(crate) async fn export(
    c: &Client,
    id: ObjectId,
    media_type: &'static str,
    released: Vec<String>,
) -> Result<ReleasedExport> {
    let mut req = c.export_text_request();
    let mut p = req.get();
    write_object_id(p.reborrow().init_id(), id);
    p.reborrow().set_media_type(media_type);
    let count = u32::try_from(released.len()).map_err(|_| unreadable("too many uids"))?;
    let mut list = p.init_released(count);
    for (i, uid) in (0u32..).zip(&released) {
        list.set(i, uid.as_str());
    }
    let answer = req
        .send()
        .promise
        .await
        .map_err(|e| error_of(&e, &person(id)))?;
    let r = answer.get().map_err(unreadable)?;
    let text = r
        .get_text()
        .and_then(|t| Ok(t.to_string()?))
        .map_err(unreadable)?;
    let withheld = r
        .get_withheld()
        .map_err(unreadable)?
        .iter()
        .map(|t| t.and_then(|t| Ok(t.to_string()?)).map_err(unreadable))
        .collect::<Result<_>>()?;
    Ok(ReleasedExport { text, withheld })
}

/// The JSON text of a card, as the server reads it.
pub(crate) fn card_text(card: &Card) -> String {
    card.to_value().to_string()
}
