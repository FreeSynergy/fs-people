//! `Persons`: cards, lookups, import and export.
//!
//! Pattern: Adapter — each function is one method of the contract and one call of the provider.

use fs_interfaces::people_capnp::persons as wire;
use fs_interfaces::rpc::MAX_PAGE_BYTES;
use fs_people_core::{ListFilter, People};

use crate::errors::{rpc_error, too_large};
use crate::formats::{exporter, importer};
use crate::wire::{
    read_card, read_filter, read_id, unpaged, write_page, write_person, write_persons, write_report,
};

type Done = Result<(), capnp::Error>;

pub(super) async fn create(
    people: &dyn People,
    params: wire::CreateParams,
    mut results: wire::CreateResults,
) -> Done {
    let card = read_card(params.get()?.get_jscontact()?)?;
    let stored = people.create(card).await.map_err(rpc_error)?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn update(
    people: &dyn People,
    params: wire::UpdateParams,
    mut results: wire::UpdateResults,
) -> Done {
    let p = params.get()?;
    let id = read_id(p.get_id()?)?;
    let card = read_card(p.get_jscontact()?)?;
    let stored = people.update(id, card).await.map_err(rpc_error)?;
    write_person(results.get().init_person(), &stored);
    Ok(())
}

pub(super) async fn delete(people: &dyn People, params: wire::DeleteParams) -> Done {
    let id = read_id(params.get()?.get_id()?)?;
    people.delete(id).await.map_err(rpc_error)
}

pub(super) async fn get(
    people: &dyn People,
    params: wire::GetParams,
    mut results: wire::GetResults,
) -> Done {
    let id = read_id(params.get()?.get_id()?)?;
    let found = people.get(id).await.map_err(rpc_error)?;
    let mut r = results.get();
    r.set_found(found.is_some());
    if let Some(stored) = found {
        write_person(r.init_person(), &stored);
    }
    Ok(())
}

pub(super) async fn get_by_uid(
    people: &dyn People,
    params: wire::GetByUidParams,
    mut results: wire::GetByUidResults,
) -> Done {
    let p = params.get()?;
    let uid = p.get_uid()?.to_str()?;
    let found = people.get_by_uid(uid).await.map_err(rpc_error)?;
    let mut r = results.get();
    r.set_found(found.is_some());
    if let Some(stored) = found {
        write_person(r.init_person(), &stored);
    }
    Ok(())
}

pub(super) async fn list(
    people: &dyn People,
    params: wire::ListParams,
    mut results: wire::ListResults,
) -> Done {
    let p = params.get()?;
    let filter = if p.has_filter() {
        read_filter(p.get_filter()?)?
    } else {
        ListFilter::default()
    };
    let all = people.list(&filter).await.map_err(rpc_error)?;
    write_page(results.get().init_page(), all, p.get_cursor())
}

pub(super) async fn find_by_describes(
    people: &dyn People,
    params: wire::FindByDescribesParams,
    mut results: wire::FindByDescribesResults,
) -> Done {
    let p = params.get()?;
    let found = people
        .find_by_describes(p.get_iri()?.to_str()?)
        .await
        .map_err(rpc_error)?;
    let length = unpaged(&found)?;
    write_persons(results.get().init_items(length), &found);
    Ok(())
}

pub(super) async fn find_by_email(
    people: &dyn People,
    params: wire::FindByEmailParams,
    mut results: wire::FindByEmailResults,
) -> Done {
    let p = params.get()?;
    let found = people
        .find_by_email(p.get_email()?.to_str()?)
        .await
        .map_err(rpc_error)?;
    let length = unpaged(&found)?;
    write_persons(results.get().init_items(length), &found);
    Ok(())
}

pub(super) async fn find_by_phone(
    people: &dyn People,
    params: wire::FindByPhoneParams,
    mut results: wire::FindByPhoneResults,
) -> Done {
    let p = params.get()?;
    let found = people
        .find_by_phone(p.get_phone()?.to_str()?)
        .await
        .map_err(rpc_error)?;
    let length = unpaged(&found)?;
    write_persons(results.get().init_items(length), &found);
    Ok(())
}

pub(super) async fn import_text(
    people: &dyn People,
    params: wire::ImportTextParams,
    mut results: wire::ImportTextResults,
) -> Done {
    let p = params.get()?;
    let format = importer(p.get_media_type()?.to_str()?)?;
    let report = people
        .import(format, p.get_text()?.to_str()?)
        .await
        .map_err(rpc_error)?;
    write_report(results.get().init_report(), &report)
}

pub(super) async fn export_text(
    people: &dyn People,
    params: wire::ExportTextParams,
    mut results: wire::ExportTextResults,
) -> Done {
    let p = params.get()?;
    let format = exporter(p.get_media_type()?.to_str()?)?;
    let id = read_id(p.get_id()?)?;
    let text = people.export(format, id).await.map_err(rpc_error)?;
    if text.len() > MAX_PAGE_BYTES {
        return Err(too_large(format!("the export is {} bytes", text.len())));
    }
    results.get().set_text(text.as_str());
    Ok(())
}
