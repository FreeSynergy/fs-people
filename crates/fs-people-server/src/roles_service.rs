//! `Roles` interface: every method of the contract, each one call of the provider.
//!
//! Pattern: Adapter — wire request in, `RoleBook` call, wire response out. An assignment leaves only
//! through the methods the contract names for it; nothing else of the stored `Relationship` does.

use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::people_capnp::roles;
use fs_people_core::RoleBook;

use crate::errors::rpc_error;
use crate::wire::{
    count, read_definition, read_details, read_id, write_assignment, write_assignments,
    write_definition, write_ids,
};

pub(crate) struct RolesImpl {
    pub(crate) roles: Arc<dyn RoleBook>,
}

type Done = Result<(), capnp::Error>;

/// Empty text is "no scope".
fn scope_of(t: &str) -> Option<&str> {
    (!t.is_empty()).then_some(t)
}

impl roles::Server for RolesImpl {
    async fn define(self: Rc<Self>, params: roles::DefineParams, _: roles::DefineResults) -> Done {
        let definition = read_definition(params.get()?.get_definition()?)?;
        self.roles.define_role(definition).await.map_err(rpc_error)
    }

    async fn undefine(
        self: Rc<Self>,
        params: roles::UndefineParams,
        _: roles::UndefineResults,
    ) -> Done {
        let p = params.get()?;
        let id = p.get_id()?.to_str()?;
        self.roles.undefine_role(id).await.map_err(rpc_error)
    }

    async fn load_config(
        self: Rc<Self>,
        params: roles::LoadConfigParams,
        _: roles::LoadConfigResults,
    ) -> Done {
        let p = params.get()?;
        let json = p.get_json()?.to_str()?;
        self.roles.load_role_config(json).await.map_err(rpc_error)
    }

    async fn get(self: Rc<Self>, params: roles::GetParams, mut results: roles::GetResults) -> Done {
        let p = params.get()?;
        let found = self
            .roles
            .role(p.get_id()?.to_str()?)
            .await
            .map_err(rpc_error)?;
        let mut r = results.get();
        r.set_found(found.is_some());
        match found {
            Some(definition) => write_definition(r.init_definition(), &definition),
            None => Ok(()),
        }
    }

    async fn list(self: Rc<Self>, _: roles::ListParams, mut results: roles::ListResults) -> Done {
        let all = self.roles.roles().await.map_err(rpc_error)?;
        let mut items = results.get().init_items(count(all.len())?);
        for (i, definition) in (0u32..).zip(&all) {
            write_definition(items.reborrow().get(i), definition)?;
        }
        Ok(())
    }

    async fn assign(
        self: Rc<Self>,
        params: roles::AssignParams,
        mut results: roles::AssignResults,
    ) -> Done {
        let p = params.get()?;
        let person = read_id(p.get_person()?)?;
        let role = p.get_role()?.to_str()?;
        let scope = scope_of(p.get_scope()?.to_str()?);
        let details = read_details(p.get_details()?)?;
        let held = self
            .roles
            .assign_role(person, role, scope, details)
            .await
            .map_err(rpc_error)?;
        write_assignment(results.get().init_assignment(), &held);
        Ok(())
    }

    async fn unassign(
        self: Rc<Self>,
        params: roles::UnassignParams,
        _: roles::UnassignResults,
    ) -> Done {
        let id = read_id(params.get()?.get_assignment()?)?;
        self.roles.unassign_role(id).await.map_err(rpc_error)
    }

    async fn roles_of(
        self: Rc<Self>,
        params: roles::RolesOfParams,
        mut results: roles::RolesOfResults,
    ) -> Done {
        let person = read_id(params.get()?.get_person()?)?;
        let held = self.roles.roles_of(person).await.map_err(rpc_error)?;
        write_assignments(results.get().init_items(count(held.len())?), &held);
        Ok(())
    }

    async fn people_with(
        self: Rc<Self>,
        params: roles::PeopleWithParams,
        mut results: roles::PeopleWithResults,
    ) -> Done {
        let p = params.get()?;
        let scope = scope_of(p.get_scope()?.to_str()?);
        let holders = self
            .roles
            .people_with_role(p.get_role()?.to_str()?, scope)
            .await
            .map_err(rpc_error)?;
        write_ids(results.get().init_ids(count(holders.len())?), &holders);
        Ok(())
    }

    async fn mark_ai(
        self: Rc<Self>,
        params: roles::MarkAiParams,
        mut results: roles::MarkAiResults,
    ) -> Done {
        let person = read_id(params.get()?.get_person()?)?;
        let held = self.roles.mark_ai(person).await.map_err(rpc_error)?;
        write_assignment(results.get().init_assignment(), &held);
        Ok(())
    }

    async fn is_ai(
        self: Rc<Self>,
        params: roles::IsAiParams,
        mut results: roles::IsAiResults,
    ) -> Done {
        let person = read_id(params.get()?.get_person()?)?;
        let marked = self.roles.is_ai(person).await.map_err(rpc_error)?;
        results.get().set_ai(marked);
        Ok(())
    }
}
