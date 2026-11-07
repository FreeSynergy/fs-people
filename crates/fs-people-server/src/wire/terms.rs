//! The SKOS terms of a role definition on the wire: `type`, `broader`, `closeMatch`, `standardRefs`.
//!
//! Pattern: Adapter — the core keeps one `RoleKind` and, so far, one address for `broader` and
//! `closeMatch`; the contract carries lists. Why: the wire stays as the contract says, and a second
//! address (which the core cannot keep) is refused instead of dropped silently.

use fs_interfaces::people_capnp::role_definition;
use fs_interfaces::rpc::invalid_request;
use fs_people_core::{RoleDefinition, RoleKind};

use super::person::count;

/// Read a list of texts.
pub(super) fn read_list(list: capnp::text_list::Reader<'_>) -> capnp::Result<Vec<String>> {
    list.iter().map(|t| Ok(t?.to_string()?)).collect()
}

fn write_list(mut out: capnp::text_list::Builder<'_>, values: &[&str]) {
    for (i, value) in (0u32..).zip(values) {
        out.set(i, value);
    }
}

/// The one address a term holds so far; a second would be dropped silently, so it is refused.
pub(super) fn single(mut values: Vec<String>, term: &str) -> capnp::Result<Option<String>> {
    if values.len() > 1 {
        return Err(invalid_request(format!("{term} holds one address so far")));
    }
    Ok(values.pop())
}

/// Empty and `skos:Concept` are the same (contract); anything else but `org:Role` is no role type.
pub(super) fn read_kind(t: capnp::text::Reader<'_>) -> capnp::Result<RoleKind> {
    match t.to_str()? {
        "" | "skos:Concept" => Ok(RoleKind::Concept),
        "org:Role" => Ok(RoleKind::OrgRole),
        other => Err(invalid_request(format!(
            "type is org:Role or skos:Concept, not {other}"
        ))),
    }
}

fn one(value: Option<&String>) -> Vec<&str> {
    value.map(String::as_str).into_iter().collect()
}

/// Write the terms of a definition.
pub(super) fn write_terms(
    mut out: role_definition::Builder<'_>,
    d: &RoleDefinition,
) -> capnp::Result<()> {
    out.set_type(match d.kind {
        RoleKind::OrgRole => "org:Role",
        RoleKind::Concept => "skos:Concept",
    });
    let broader = one(d.broader.as_ref());
    write_list(out.reborrow().init_broader(count(broader.len())?), &broader);
    let close = one(d.close_match.as_ref());
    write_list(out.reborrow().init_close_match(count(close.len())?), &close);
    let refs: Vec<&str> = d.standard_refs.iter().map(String::as_str).collect();
    write_list(out.reborrow().init_standard_refs(count(refs.len())?), &refs);
    Ok(())
}
