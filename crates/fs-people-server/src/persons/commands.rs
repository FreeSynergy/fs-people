//! `Persons`: the commands of the description `Person` that are no plain card call: patch, check, suggest.
//!
//! Pattern: Adapter — each function is one method of the contract and one call of the provider.

use std::sync::Arc;

use fs_interfaces::people_capnp::persons as wire;
use fs_interfaces::rpc::{check_answer, check_intake, invalid_request, read_object_id};
use fs_people_core::validate::check_card_numbers;
use fs_people_core::{PatchObject, People};

use crate::errors::{finish, rpc_error};
use crate::wire::{count, intake, read_edit, required, write_person};

type Done = Result<(), capnp::Error>;

pub(super) async fn patch(
    people: &Arc<dyn People>,
    params: wire::PatchParams,
    mut results: wire::PatchResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let text = intake(p.get_patch()?, "patch")?;
    check_card_numbers(text).map_err(|e| invalid_request(e.to_string()))?;
    let patch = PatchObject::parse(text).map_err(rpc_error)?;
    let people = people.clone();
    let stored = finish(async move { people.patch(id, &patch).await }).await?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn change(
    people: &Arc<dyn People>,
    params: wire::ChangeParams,
    mut results: wire::ChangeResults,
) -> Done {
    let p = params.get()?;
    let id = read_object_id(p.get_id()?)?;
    let edit = read_edit(p.get_edit()?)?;
    let people = people.clone();
    let stored = finish(async move { people.change(id, &edit).await }).await?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn check(
    people: &Arc<dyn People>,
    params: wire::CheckParams,
    mut results: wire::CheckResults,
) -> Done {
    let id = read_object_id(params.get()?.get_id()?)?;
    let problems = people.check(id).await.map_err(rpc_error)?;
    let weight: usize = problems
        .iter()
        .map(|i| i.path.len() + i.message.len() + 32)
        .sum();
    check_answer(weight)?;
    let mut out = results.get().init_problems(count(problems.len())?);
    for (i, issue) in (0u32..).zip(&problems) {
        let mut one = out.reborrow().get(i);
        one.set_path(issue.path.as_str());
        one.set_code(issue.message.as_str());
    }
    Ok(())
}

pub(super) async fn suggest(
    people: &Arc<dyn People>,
    params: wire::SuggestParams,
    mut results: wire::SuggestResults,
) -> Done {
    let p = params.get()?;
    let field = required(p.get_field()?, "field")?;
    // What is typed may be a single letter, so it is not `required`; it is bounded like any text.
    let typed = p.get_typed()?.to_str()?;
    check_intake(typed.len())?;
    let found = people.suggest(field, typed).await.map_err(rpc_error)?;
    check_answer(found.iter().map(|s| s.len() + 16).sum())?;
    let mut out = results.get().init_suggestions(count(found.len())?);
    for (i, s) in (0u32..).zip(&found) {
        out.set(i, s.as_str());
    }
    Ok(())
}
