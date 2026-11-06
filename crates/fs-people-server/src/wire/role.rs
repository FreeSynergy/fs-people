//! Roles on the wire.
//!
//! Pattern: Adapter — `RoleDefinition` and `RoleAssignment` between Rust and `people.capnp`. Empty
//! text means "none". The `extra` members of a definition have no place on the wire: only roles in a
//! configuration keep them, `DESIGN.md` names the gap.

use std::collections::BTreeMap;

use fs_interfaces::people_capnp::{
    RoleScope as Wire, localized, role_assignment, role_definition, role_field,
};
use fs_interfaces::rpc::write_object_id;
use fs_people_core::{RoleAssignment, RoleDefinition, RoleField, RoleScope};
use serde_json::{Map, Value};

use super::person::count;
use crate::errors::invalid;

type Texts = BTreeMap<String, String>;

fn optional(t: capnp::text::Reader<'_>) -> capnp::Result<Option<String>> {
    let s = t.to_string()?;
    Ok((!s.is_empty()).then_some(s))
}

fn read_texts(list: capnp::struct_list::Reader<'_, localized::Owned>) -> capnp::Result<Texts> {
    list.iter()
        .map(|l| Ok((l.get_language()?.to_string()?, l.get_text()?.to_string()?)))
        .collect()
}

fn write_texts(mut out: capnp::struct_list::Builder<'_, localized::Owned>, texts: &Texts) {
    for (i, (language, text)) in (0u32..).zip(texts) {
        let mut item = out.reborrow().get(i);
        item.set_language(language.as_str());
        item.set_text(text.as_str());
    }
}

fn read_field(r: role_field::Reader<'_>) -> capnp::Result<RoleField> {
    Ok(RoleField {
        name: r.get_name()?.to_string()?,
        standard: optional(r.get_standard()?)?,
        description: read_texts(r.get_description()?)?,
    })
}

fn write_field(mut out: role_field::Builder<'_>, f: &RoleField) -> capnp::Result<()> {
    out.set_name(f.name.as_str());
    out.set_standard(f.standard.as_deref().unwrap_or(""));
    write_texts(
        out.init_description(count(f.description.len())?),
        &f.description,
    );
    Ok(())
}

/// Read a definition from a request.
pub(crate) fn read_definition(r: role_definition::Reader<'_>) -> capnp::Result<RoleDefinition> {
    Ok(RoleDefinition {
        id: r.get_id()?.to_string()?,
        names: read_texts(r.get_names()?)?,
        description: read_texts(r.get_description()?)?,
        standard: optional(r.get_standard()?)?,
        fields: r
            .get_fields()?
            .iter()
            .map(read_field)
            .collect::<capnp::Result<_>>()?,
        owner: r.get_owner()?.to_string()?,
        scope: match r.get_scope()? {
            Wire::Personal => RoleScope::Personal,
            Wire::Group => RoleScope::Group,
            Wire::Public => RoleScope::Public,
        },
        card_kind: optional(r.get_card_kind()?)?,
        ..RoleDefinition::default()
    })
}

/// Write a definition.
pub(crate) fn write_definition(
    mut out: role_definition::Builder<'_>,
    d: &RoleDefinition,
) -> capnp::Result<()> {
    out.set_id(d.id.as_str());
    write_texts(out.reborrow().init_names(count(d.names.len())?), &d.names);
    write_texts(
        out.reborrow().init_description(count(d.description.len())?),
        &d.description,
    );
    out.set_standard(d.standard.as_deref().unwrap_or(""));
    let mut fields = out.reborrow().init_fields(count(d.fields.len())?);
    for (i, f) in (0u32..).zip(&d.fields) {
        write_field(fields.reborrow().get(i), f)?;
    }
    out.set_owner(d.owner.as_str());
    out.set_scope(match d.scope {
        RoleScope::Personal => Wire::Personal,
        RoleScope::Group => Wire::Group,
        RoleScope::Public => Wire::Public,
    });
    out.set_card_kind(d.card_kind.as_deref().unwrap_or(""));
    Ok(())
}

/// Read the details of an assignment: a JSON object; empty text is an empty object.
pub(crate) fn read_details(t: capnp::text::Reader<'_>) -> capnp::Result<Map<String, Value>> {
    let text = t.to_str()?;
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    serde_json::from_str(text).map_err(|e| invalid(format!("details are no JSON object: {e}")))
}

/// Write one assignment.
pub(crate) fn write_assignment(mut out: role_assignment::Builder<'_>, a: &RoleAssignment) {
    write_object_id(out.reborrow().init_id(), a.id);
    write_object_id(out.reborrow().init_person(), a.person);
    out.set_role(a.role.as_str());
    out.set_scope(a.scope.as_deref().unwrap_or(""));
    out.set_details(Value::Object(a.details.clone()).to_string().as_str());
}

/// Write a list of assignments.
pub(crate) fn write_assignments(
    mut out: capnp::struct_list::Builder<'_, role_assignment::Owned>,
    found: &[RoleAssignment],
) {
    for (i, a) in (0u32..).zip(found) {
        write_assignment(out.reborrow().get(i), a);
    }
}
