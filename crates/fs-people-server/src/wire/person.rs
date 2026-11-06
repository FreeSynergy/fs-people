//! People, pages and reports on the wire.
//!
//! Pattern: Adapter — `StoredObject`, `ImportReport` and ids between Rust and `people.capnp`. The
//! JSContact JSON stays authoritative for the content; the envelope is the routing summary.
//!
//! `Person.roles` holds "only what the caller may see" (ADR 0029). This server does not know the
//! caller beyond its user id, so it stays empty; roles are asked for on `Roles`. Provisional: role
//! privacy is open with the strategy session (`DESIGN.md`).

use fs_interfaces::common_capnp;
use fs_interfaces::people_capnp::{
    person, person_filter, person_page, relation, same_person_proposal,
};
use fs_interfaces::rpc::{
    DEFAULT_PAGE_SIZE, Page, check_answer, check_intake, envelope_weight, invalid_request,
    write_envelope, write_object_id,
};
use fs_people_core::validate::check_exact_numbers;
use fs_people_core::{Card, ListFilter, MatchKind, SameAsProposal};
use fs_store::{ObjectId, StoredObject};

/// Room for one list entry with its pointers; only for the check that an answer fits.
pub(super) const UUID_BYTES: usize = 32;

/// A length as the wire counts it.
pub(crate) fn count(n: usize) -> capnp::Result<u32> {
    u32::try_from(n).map_err(|_| capnp::Error::failed("list too long".into()))
}

/// A text the request must not leave empty.
pub(crate) fn required<'a>(t: capnp::text::Reader<'a>, what: &str) -> capnp::Result<&'a str> {
    let text = t.to_str()?;
    if text.trim().is_empty() {
        return Err(invalid_request(format!("{what} is empty")));
    }
    Ok(text)
}

/// A required text that gets stored: it also has to fit an answer later.
pub(crate) fn intake<'a>(t: capnp::text::Reader<'a>, what: &str) -> capnp::Result<&'a str> {
    let text = required(t, what)?;
    check_intake(text.len())?;
    Ok(text)
}

/// What one person costs in an answer.
fn weight(o: &StoredObject) -> usize {
    o.content.to_string().len() + envelope_weight(&o.envelope)
}

/// Write one person.
pub(crate) fn write_person(mut out: person::Builder<'_>, o: &StoredObject) {
    write_envelope(out.reborrow().init_envelope(), &o.envelope);
    out.set_jscontact(o.content.to_string().as_str());
}

/// Read the JSContact text of a request.
pub(crate) fn read_card(text: capnp::text::Reader<'_>) -> capnp::Result<Card> {
    let text = intake(text, "jscontact")?;
    let card = serde_json::from_str(text).map_err(|e| invalid_request(e.to_string()))?;
    check_exact_numbers(text).map_err(|e| invalid_request(e.to_string()))?;
    Ok(card)
}

/// Read a filter: empty members are no restriction.
pub(crate) fn read_filter(r: person_filter::Reader<'_>) -> capnp::Result<ListFilter> {
    let given = |t: capnp::text::Reader<'_>| -> capnp::Result<Option<String>> {
        let s = t.to_string()?;
        Ok((!s.trim().is_empty()).then_some(s))
    };
    Ok(ListFilter {
        kind: given(r.get_kind()?)?,
        role: given(r.get_role()?)?,
        term: given(r.get_term()?)?,
        ..ListFilter::default()
    })
}

/// Cut the page at `cursor` and write it.
pub(crate) fn write_page(
    mut out: person_page::Builder<'_>,
    all: Vec<StoredObject>,
    cursor: capnp::Result<common_capnp::cursor::Reader<'_>>,
) -> capnp::Result<()> {
    let opaque = cursor?.get_opaque()?;
    let page = Page::of_weighted(all, opaque, DEFAULT_PAGE_SIZE, weight)?;
    let mut items = out.reborrow().init_items(count(page.items.len())?);
    for (i, o) in (0u32..).zip(&page.items) {
        write_person(items.reborrow().get(i), o);
    }
    out.init_next().set_opaque(&page.next);
    Ok(())
}

/// How many people an answer without a cursor will hold. All or nothing: it has no next page.
pub(crate) fn unpaged(found: &[StoredObject]) -> capnp::Result<u32> {
    check_answer(found.iter().map(weight).sum())?;
    count(found.len())
}

/// Write a whole list, for answers without a cursor.
pub(crate) fn write_persons(
    mut out: capnp::struct_list::Builder<'_, person::Owned>,
    found: &[StoredObject],
) {
    for (i, o) in (0u32..).zip(found) {
        write_person(out.reborrow().get(i), o);
    }
}

/// Write a list of ids.
pub(crate) fn write_ids(
    mut out: capnp::struct_list::Builder<'_, common_capnp::uuid::Owned>,
    ids: &[ObjectId],
) {
    for (i, id) in (0u32..).zip(ids) {
        write_object_id(out.reborrow().get(i), *id);
    }
}

/// Write the relations of a card.
pub(crate) fn write_relations(
    mut out: capnp::struct_list::Builder<'_, relation::Owned>,
    relations: &[(String, String)],
) {
    for (i, (uid, kind)) in (0u32..).zip(relations) {
        let mut item = out.reborrow().get(i);
        item.set_uid(uid.as_str());
        item.set_relation(kind.as_str());
    }
}

/// How many ids an answer will hold, if they fit one.
pub(crate) fn ids_count(ids: &[ObjectId]) -> capnp::Result<u32> {
    check_answer(ids.len().saturating_mul(UUID_BYTES))?;
    count(ids.len())
}

/// How many relations an answer will hold, if they fit one.
pub(crate) fn relations_count(relations: &[(String, String)]) -> capnp::Result<u32> {
    check_answer(
        relations
            .iter()
            .map(|(u, r)| u.len() + r.len() + UUID_BYTES)
            .sum(),
    )?;
    count(relations.len())
}

/// How many proposals an answer will hold, if they fit one: people sharing one address make many.
pub(crate) fn proposals_count(proposals: &[SameAsProposal]) -> capnp::Result<u32> {
    let reasons = |p: &SameAsProposal| -> usize {
        p.reasons.iter().map(|r| r.value.len() + UUID_BYTES).sum()
    };
    check_answer(proposals.iter().map(|p| reasons(p) + 3 * UUID_BYTES).sum())?;
    count(proposals.len())
}

/// Write same-person proposals; a reason reads `email:<value>` or `phone:<value>`.
pub(crate) fn write_proposals(
    mut out: capnp::struct_list::Builder<'_, same_person_proposal::Owned>,
    proposals: &[SameAsProposal],
) -> capnp::Result<()> {
    for (i, p) in (0u32..).zip(proposals) {
        let mut item = out.reborrow().get(i);
        write_object_id(item.reborrow().init_a(), p.a);
        write_object_id(item.reborrow().init_b(), p.b);
        let mut reasons = item.init_reasons(count(p.reasons.len())?);
        for (j, r) in (0u32..).zip(&p.reasons) {
            let kind = match r.kind {
                MatchKind::Email => "email",
                MatchKind::Phone => "phone",
            };
            reasons.set(j, format!("{kind}:{}", r.value).as_str());
        }
    }
    Ok(())
}
