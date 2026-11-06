//! The dispatch of `Persons`: every method of the contract, in its order.
//!
//! Pattern: Adapter — wire request in, `People` call, wire response out; the functions behind it do
//! that per concern. What a provider cannot do arrives as its explicit `Unsupported`.

use std::rc::Rc;
use std::sync::Arc;

use fs_interfaces::people_capnp::persons as wire;
use fs_people_core::People;

use super::{cards, links, lists};

pub(crate) struct PersonsImpl {
    pub(crate) people: Arc<dyn People>,
}

type Done = Result<(), capnp::Error>;

impl wire::Server for PersonsImpl {
    async fn create(self: Rc<Self>, p: wire::CreateParams, r: wire::CreateResults) -> Done {
        cards::create(&self.people, p, r).await
    }
    async fn update(self: Rc<Self>, p: wire::UpdateParams, r: wire::UpdateResults) -> Done {
        cards::update(&self.people, p, r).await
    }
    async fn delete(self: Rc<Self>, p: wire::DeleteParams, _: wire::DeleteResults) -> Done {
        cards::delete(&self.people, p).await
    }
    async fn get(self: Rc<Self>, p: wire::GetParams, r: wire::GetResults) -> Done {
        cards::get(&self.people, p, r).await
    }
    async fn get_by_uid(self: Rc<Self>, p: wire::GetByUidParams, r: wire::GetByUidResults) -> Done {
        cards::get_by_uid(&self.people, p, r).await
    }
    async fn list(self: Rc<Self>, p: wire::ListParams, r: wire::ListResults) -> Done {
        cards::list(&self.people, p, r).await
    }
    async fn find_by_describes(
        self: Rc<Self>,
        p: wire::FindByDescribesParams,
        r: wire::FindByDescribesResults,
    ) -> Done {
        cards::find_by_describes(&self.people, p, r).await
    }
    async fn find_by_email(
        self: Rc<Self>,
        p: wire::FindByEmailParams,
        r: wire::FindByEmailResults,
    ) -> Done {
        cards::find_by_email(&self.people, p, r).await
    }
    async fn find_by_phone(
        self: Rc<Self>,
        p: wire::FindByPhoneParams,
        r: wire::FindByPhoneResults,
    ) -> Done {
        cards::find_by_phone(&self.people, p, r).await
    }
    async fn import_text(
        self: Rc<Self>,
        p: wire::ImportTextParams,
        r: wire::ImportTextResults,
    ) -> Done {
        cards::import_text(&self.people, p, r).await
    }
    async fn export_text(
        self: Rc<Self>,
        p: wire::ExportTextParams,
        r: wire::ExportTextResults,
    ) -> Done {
        cards::export_text(&self.people, p, r).await
    }
    async fn relate(self: Rc<Self>, p: wire::RelateParams, r: wire::RelateResults) -> Done {
        links::relate(&self.people, p, r).await
    }
    async fn unrelate(self: Rc<Self>, p: wire::UnrelateParams, r: wire::UnrelateResults) -> Done {
        links::unrelate(&self.people, p, r).await
    }
    async fn relations(
        self: Rc<Self>,
        p: wire::RelationsParams,
        r: wire::RelationsResults,
    ) -> Done {
        links::relations(&self.people, p, r).await
    }
    async fn proposals(
        self: Rc<Self>,
        _: wire::ProposalsParams,
        r: wire::ProposalsResults,
    ) -> Done {
        links::proposals(&self.people, r).await
    }
    async fn confirm_same(
        self: Rc<Self>,
        p: wire::ConfirmSameParams,
        r: wire::ConfirmSameResults,
    ) -> Done {
        links::confirm_same(&self.people, p, r).await
    }
    async fn unlink_same(
        self: Rc<Self>,
        p: wire::UnlinkSameParams,
        _: wire::UnlinkSameResults,
    ) -> Done {
        links::unlink_same(&self.people, p).await
    }
    async fn linked_to(self: Rc<Self>, p: wire::LinkedToParams, r: wire::LinkedToResults) -> Done {
        links::linked_to(&self.people, p, r).await
    }
    async fn create_list(
        self: Rc<Self>,
        p: wire::CreateListParams,
        r: wire::CreateListResults,
    ) -> Done {
        lists::create_list(&self.people, p, r).await
    }
    async fn add_to_list(
        self: Rc<Self>,
        p: wire::AddToListParams,
        r: wire::AddToListResults,
    ) -> Done {
        lists::add_to_list(&self.people, p, r).await
    }
    async fn remove_from_list(
        self: Rc<Self>,
        p: wire::RemoveFromListParams,
        r: wire::RemoveFromListResults,
    ) -> Done {
        lists::remove_from_list(&self.people, p, r).await
    }
    async fn list_members(
        self: Rc<Self>,
        p: wire::ListMembersParams,
        r: wire::ListMembersResults,
    ) -> Done {
        lists::list_members(&self.people, p, r).await
    }
    async fn lists_of(self: Rc<Self>, p: wire::ListsOfParams, r: wire::ListsOfResults) -> Done {
        lists::lists_of(&self.people, p, r).await
    }
}
