//! The person calls against the server.
//!
//! Pattern: Adapter — each core operation as a request on the server's persons capability.

use fs_interfaces::common_capnp;
use fs_interfaces::people_capnp::persons;
use fs_interfaces::rpc::write_uuid;
use fs_objects::{ObjectId, StoredObject};
use fs_people_core::{ListFilter, Result};

use super::call::Revisions;
use super::convert::{problem, stored};

pub(super) async fn get(
    persons: &persons::Client,
    revisions: &Revisions,
    id: ObjectId,
) -> Result<Option<StoredObject>> {
    let mut request = persons.get_request();
    write_uuid(request.get().init_id(), Some(id.as_uuid()));
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, Some(id)))?;
    let r = answer.get().map_err(|e| problem(&e, Some(id)))?;
    if !r.get_found() {
        return Ok(None);
    }
    let person = r
        .get_person()
        .and_then(|p| stored(revisions, p))
        .map_err(|e| problem(&e, Some(id)))?;
    Ok(Some(person))
}

pub(super) async fn list(
    persons: &persons::Client,
    revisions: &Revisions,
    filter: &ListFilter,
) -> Result<Vec<StoredObject>> {
    let (mut found, mut cursor) = (Vec::new(), Vec::<u8>::new());
    let mut skip = filter.offset;
    if filter.limit == Some(0) {
        return Ok(found);
    }
    loop {
        let mut request = persons.list_request();
        {
            let mut wire = request.get().init_filter();
            if let Some(term) = &filter.term {
                wire.set_term(term.as_str());
            }
            if let Some(kind) = &filter.kind {
                wire.set_kind(kind.as_str());
            }
            if let Some(role) = &filter.role {
                wire.set_role(role.as_str());
            }
        }
        request.get().init_cursor().set_opaque(&cursor);
        let answer = request
            .send()
            .promise
            .await
            .map_err(|e| problem(&e, None))?;
        let page = answer
            .get()
            .and_then(persons::list_results::Reader::get_page)
            .map_err(|e| problem(&e, None))?;
        for item in page.get_items().map_err(|e| problem(&e, None))? {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            found.push(stored(revisions, item).map_err(|e| problem(&e, None))?);
            if filter.limit.is_some_and(|limit| found.len() >= limit) {
                return Ok(found);
            }
        }
        let next = page
            .get_next()
            .and_then(common_capnp::cursor::Reader::get_opaque)
            .map_err(|e| problem(&e, None))?;
        if next.is_empty() {
            return Ok(found);
        }
        cursor = next.to_vec();
    }
}

pub(super) async fn create(
    persons: &persons::Client,
    revisions: &Revisions,
    card: &str,
) -> Result<StoredObject> {
    let mut request = persons.create_request();
    request.get().set_jscontact(card);
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, None))?;
    answer
        .get()
        .and_then(persons::create_results::Reader::get_person)
        .and_then(|p| stored(revisions, p))
        .map_err(|e| problem(&e, None))
}

pub(super) async fn update(
    persons: &persons::Client,
    revisions: &Revisions,
    id: ObjectId,
    card: &str,
) -> Result<StoredObject> {
    let mut request = persons.update_request();
    write_uuid(request.get().init_id(), Some(id.as_uuid()));
    request.get().set_jscontact(card);
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, Some(id)))?;
    answer
        .get()
        .and_then(persons::update_results::Reader::get_person)
        .and_then(|p| stored(revisions, p))
        .map_err(|e| problem(&e, Some(id)))
}

pub(super) async fn update_if(
    persons: &persons::Client,
    revisions: &Revisions,
    id: ObjectId,
    revision: u64,
    card: &str,
) -> Result<StoredObject> {
    let mut request = persons.update_if_request();
    write_uuid(request.get().init_id(), Some(id.as_uuid()));
    request.get().set_revision(revision);
    request.get().set_jscontact(card);
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, Some(id)))?;
    answer
        .get()
        .and_then(persons::update_if_results::Reader::get_person)
        .and_then(|p| stored(revisions, p))
        .map_err(|e| problem(&e, Some(id)))
}

pub(super) async fn delete(persons: &persons::Client, id: ObjectId) -> Result<()> {
    let mut request = persons.delete_request();
    write_uuid(request.get().init_id(), Some(id.as_uuid()));
    request
        .send()
        .promise
        .await
        .map(|_| ())
        .map_err(|e| problem(&e, Some(id)))
}
