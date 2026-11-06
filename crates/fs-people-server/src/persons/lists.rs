//! `Persons`: JSContact group cards as lists of people. Rights and belonging are `fs-groups` (ADR 0050).
//!
//! Pattern: Adapter — each function is one method of the contract and one call of the provider.

use fs_interfaces::people_capnp::persons as wire;
use fs_people_core::People;

use crate::errors::rpc_error;
use crate::wire::{read_id, unpaged, write_person, write_persons};

type Done = Result<(), capnp::Error>;

pub(super) async fn create_list(
    people: &dyn People,
    params: wire::CreateListParams,
    mut results: wire::CreateListResults,
) -> Done {
    let p = params.get()?;
    let name = p.get_name()?.to_str()?;
    let stored = people.create_group(name).await.map_err(rpc_error)?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn add_to_list(
    people: &dyn People,
    params: wire::AddToListParams,
    mut results: wire::AddToListResults,
) -> Done {
    let p = params.get()?;
    let (list, member) = (read_id(p.get_list()?)?, read_id(p.get_member()?)?);
    let stored = people.add_member(list, member).await.map_err(rpc_error)?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn remove_from_list(
    people: &dyn People,
    params: wire::RemoveFromListParams,
    mut results: wire::RemoveFromListResults,
) -> Done {
    let p = params.get()?;
    let (list, member) = (read_id(p.get_list()?)?, read_id(p.get_member()?)?);
    let stored = people
        .remove_member(list, member)
        .await
        .map_err(rpc_error)?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn list_members(
    people: &dyn People,
    params: wire::ListMembersParams,
    mut results: wire::ListMembersResults,
) -> Done {
    let list = read_id(params.get()?.get_list()?)?;
    let found = people.members(list).await.map_err(rpc_error)?;
    let length = unpaged(&found)?;
    write_persons(results.get().init_items(length), &found);
    Ok(())
}

pub(super) async fn lists_of(
    people: &dyn People,
    params: wire::ListsOfParams,
    mut results: wire::ListsOfResults,
) -> Done {
    let member = read_id(params.get()?.get_member()?)?;
    let found = people.groups_of(member).await.map_err(rpc_error)?;
    let length = unpaged(&found)?;
    write_persons(results.get().init_items(length), &found);
    Ok(())
}
