//! `Persons`: JSContact group cards as lists of people. Rights and belonging are `fs-groups` (ADR 0050).
//!
//! Pattern: Adapter — each function is one method of the contract and one call of the provider.

use std::sync::Arc;

use fs_interfaces::people_capnp::persons as wire;
use fs_interfaces::rpc::read_object_id;
use fs_people_core::People;

use crate::errors::{finish, rpc_error};
use crate::wire::{intake, unpaged, write_person, write_persons};

type Done = Result<(), capnp::Error>;

pub(super) async fn create_list(
    people: &Arc<dyn People>,
    params: wire::CreateListParams,
    mut results: wire::CreateListResults,
) -> Done {
    let p = params.get()?;
    let name = intake(p.get_name()?, "name")?.to_string();
    let people = people.clone();
    let stored = finish(async move { people.create_group(&name).await }).await?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn add_to_list(
    people: &Arc<dyn People>,
    params: wire::AddToListParams,
    mut results: wire::AddToListResults,
) -> Done {
    let p = params.get()?;
    let (list, member) = (
        read_object_id(p.get_list()?)?,
        read_object_id(p.get_member()?)?,
    );
    let people = people.clone();
    let stored = finish(async move { people.add_member(list, member).await }).await?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn remove_from_list(
    people: &Arc<dyn People>,
    params: wire::RemoveFromListParams,
    mut results: wire::RemoveFromListResults,
) -> Done {
    let p = params.get()?;
    let (list, member) = (
        read_object_id(p.get_list()?)?,
        read_object_id(p.get_member()?)?,
    );
    let people = people.clone();
    let stored = finish(async move { people.remove_member(list, member).await }).await?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn list_members(
    people: &Arc<dyn People>,
    params: wire::ListMembersParams,
    mut results: wire::ListMembersResults,
) -> Done {
    let list = read_object_id(params.get()?.get_list()?)?;
    let found = people.members(list).await.map_err(rpc_error)?;
    let length = unpaged(&found)?;
    write_persons(results.get().init_items(length), &found);
    Ok(())
}

pub(super) async fn lists_of(
    people: &Arc<dyn People>,
    params: wire::ListsOfParams,
    mut results: wire::ListsOfResults,
) -> Done {
    let member = read_object_id(params.get()?.get_member()?)?;
    let found = people.groups_of(member).await.map_err(rpc_error)?;
    let length = unpaged(&found)?;
    write_persons(results.get().init_items(length), &found);
    Ok(())
}
