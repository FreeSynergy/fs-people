//! `Persons`: relations and same-person links.
//!
//! Pattern: Adapter — each function is one method of the contract and one call of the provider. Of
//! the stored `Relationship` behind a same-person link only its id leaves, as the contract names it.

use fs_interfaces::people_capnp::persons as wire;
use fs_interfaces::rpc::{read_object_id, write_object_id};
use fs_people_core::People;

use crate::errors::rpc_error;
use crate::wire::{
    ids_count, intake, proposals_count, relations_count, write_ids, write_person, write_proposals,
    write_relations,
};

type Done = Result<(), capnp::Error>;

pub(super) async fn relate(
    people: &dyn People,
    params: wire::RelateParams,
    mut results: wire::RelateResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let uid = intake(p.get_other_uid()?, "otherUid")?;
    let relation = intake(p.get_relation()?, "relation")?;
    let stored = people.relate(id, uid, relation).await.map_err(rpc_error)?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn unrelate(
    people: &dyn People,
    params: wire::UnrelateParams,
    mut results: wire::UnrelateResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let uid = intake(p.get_other_uid()?, "otherUid")?;
    let relation = intake(p.get_relation()?, "relation")?;
    let stored = people
        .unrelate(id, uid, relation)
        .await
        .map_err(rpc_error)?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn relations(
    people: &dyn People,
    params: wire::RelationsParams,
    mut results: wire::RelationsResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let found = people.relations(id).await.map_err(rpc_error)?;
    write_relations(results.get().init_items(relations_count(&found)?), &found);
    Ok(())
}

pub(super) async fn proposals(people: &dyn People, mut results: wire::ProposalsResults) -> Done {
    let found = people
        .find_same_person_proposals()
        .await
        .map_err(rpc_error)?;
    write_proposals(results.get().init_items(proposals_count(&found)?), &found)
}

pub(super) async fn confirm_same(
    people: &dyn People,
    params: wire::ConfirmSameParams,
    mut results: wire::ConfirmSameResults,
) -> Done {
    let p = params.get()?;
    let (a, b) = (read_object_id(p.get_a()?)?, read_object_id(p.get_b()?)?);
    let link = people.confirm_same_person(a, b).await.map_err(rpc_error)?;
    write_object_id(results.get().init_link(), link.envelope.id);
    Ok(())
}

pub(super) async fn unlink_same(people: &dyn People, params: wire::UnlinkSameParams) -> Done {
    let link = read_object_id(params.get()?.get_link()?)?;
    people.unlink(link).await.map_err(rpc_error)
}

pub(super) async fn linked_to(
    people: &dyn People,
    params: wire::LinkedToParams,
    mut results: wire::LinkedToResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let found = people.linked_to(id).await.map_err(rpc_error)?;
    write_ids(results.get().init_ids(ids_count(&found)?), &found);
    Ok(())
}
