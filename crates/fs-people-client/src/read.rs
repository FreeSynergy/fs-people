//! What the server sends, as the values of the core.
//!
//! Pattern: Adapter — `Person`, `RoleAssignment`, `RoleDefinition` and `ImportReport` of
//! `people.capnp` to `StoredObject`, `RoleAssignment`, `RoleDefinition` and `ImportReport`. A reply
//! that cannot be read is `Error::Stored`: the request is not to blame.

use std::collections::BTreeMap;

use fs_interfaces::people_capnp::{
    RoleScope as Wire, import_report, localized, person, role_assignment, role_definition,
    role_field,
};
use fs_interfaces::rpc::{read_envelope, read_object_id};
use fs_people_core::{
    Error, ImportReport, Result, RoleAssignment, RoleDefinition, RoleField, RoleScope,
};
use fs_store::StoredObject;
use serde_json::Value;

/// A reply that does not read.
pub(crate) fn unreadable(e: impl std::fmt::Display) -> Error {
    Error::Stored(format!("the people server sent something unreadable: {e}"))
}

fn wire<T>(r: capnp::Result<T>) -> Result<T> {
    r.map_err(unreadable)
}

fn text(t: capnp::Result<capnp::text::Reader<'_>>) -> Result<String> {
    wire(t.and_then(|t| Ok(t.to_string()?)))
}

fn optional(t: capnp::Result<capnp::text::Reader<'_>>) -> Result<Option<String>> {
    Ok(Some(text(t)?).filter(|s| !s.is_empty()))
}

/// A person: the envelope as sent, the JSContact text as `content`.
///
/// Not on the wire, so not here: the `original` of an import (ADR 0030) and the index fields.
pub(crate) fn stored(r: person::Reader<'_>) -> Result<StoredObject> {
    let envelope = wire(r.get_envelope().and_then(read_envelope))?;
    let json = text(r.get_jscontact())?;
    let content: Value = serde_json::from_str(&json).map_err(unreadable)?;
    Ok(StoredObject::new(
        envelope,
        "application/jscontact+json",
        content,
    ))
}

/// An assignment.
pub(crate) fn assignment(r: role_assignment::Reader<'_>) -> Result<RoleAssignment> {
    let details = text(r.get_details())?;
    let details = if details.trim().is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_str(&details).map_err(unreadable)?
    };
    let Value::Object(details) = details else {
        return Err(unreadable("details are no JSON object"));
    };
    Ok(RoleAssignment {
        id: wire(r.get_id().and_then(read_object_id))?,
        person: wire(r.get_person().and_then(read_object_id))?,
        role: text(r.get_role())?,
        scope: optional(r.get_scope())?,
        details,
    })
}

fn texts(
    list: capnp::Result<capnp::struct_list::Reader<'_, localized::Owned>>,
) -> Result<BTreeMap<String, String>> {
    wire(list)?
        .iter()
        .map(|l| Ok((text(l.get_language())?, text(l.get_text())?)))
        .collect()
}

fn field(r: role_field::Reader<'_>) -> Result<RoleField> {
    Ok(RoleField {
        name: text(r.get_name())?,
        standard: optional(r.get_standard())?,
        description: texts(r.get_description())?,
    })
}

/// A role definition. Its `extra` members have no place on the wire (`DESIGN.md`).
pub(crate) fn definition(r: role_definition::Reader<'_>) -> Result<RoleDefinition> {
    let scope = match r.get_scope().map_err(unreadable)? {
        Wire::Personal => RoleScope::Personal,
        Wire::Group => RoleScope::Group,
        Wire::Public => RoleScope::Public,
    };
    Ok(RoleDefinition {
        id: text(r.get_id())?,
        names: texts(r.get_names())?,
        description: texts(r.get_description())?,
        standard: optional(r.get_standard())?,
        fields: wire(r.get_fields())?
            .iter()
            .map(field)
            .collect::<Result<_>>()?,
        owner: text(r.get_owner())?,
        scope,
        card_kind: optional(r.get_card_kind())?,
        ..RoleDefinition::default()
    })
}

fn list(l: capnp::Result<capnp::text_list::Reader<'_>>) -> Result<Vec<String>> {
    wire(l)?.iter().map(text).collect()
}

/// The report of an import.
pub(crate) fn report(r: import_report::Reader<'_>) -> Result<ImportReport> {
    Ok(ImportReport {
        created: wire(r.get_created())?
            .iter()
            .map(|id| wire(read_object_id(id)))
            .collect::<Result<_>>()?,
        skipped: list(r.get_skipped())?,
        failed: list(r.get_failed())?,
        unclear: list(r.get_unclear())?,
    })
}
