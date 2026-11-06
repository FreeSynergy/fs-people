//! The provider rule (ADR 0045) through the wire: what a provider does not offer says so.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use common::{NOT_POSSIBLE, card, expect_problem, persons_of, roles_of, with_root};
use fs_interfaces::Capabilities;
use fs_people_core::{ListFilter, People, PeopleFeed, Result as CoreResult, RoleBook};
use fs_people_server::PeopleServer;
use fs_store::{ObjectId, StoredObject};

/// A provider of cards alone: real for the provider rule, which only such a one can show.
struct ReadOnly;

#[async_trait]
impl People for ReadOnly {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new("read-only");
        c.supported("person.read");
        c.unsupported("person.create", "this source cannot be written");
        c
    }
    async fn get(&self, _: ObjectId) -> CoreResult<Option<StoredObject>> {
        Ok(None)
    }
    async fn list(&self, _: &ListFilter) -> CoreResult<Vec<StoredObject>> {
        Ok(vec![])
    }
}

#[async_trait]
impl RoleBook for ReadOnly {}
impl PeopleFeed for ReadOnly {}

#[tokio::test(flavor = "current_thread")]
async fn what_a_provider_does_not_offer_says_so() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let provider = Arc::new(ReadOnly);
            let server = PeopleServer::new(provider.clone(), provider.clone(), provider);
            with_root(server.into_client(), |root| async move {
                let persons = persons_of(&root).await;
                let answer = persons.list_request().send().promise.await.unwrap();
                let page = answer.get().unwrap().get_page().unwrap();
                assert_eq!(page.get_items().unwrap().len(), 0);

                let mut create = persons.create_request();
                create
                    .get()
                    .set_jscontact(card("u", "Ada", "a@x.org").as_str());
                let detail = expect_problem(create.send().promise.await, NOT_POSSIBLE);
                assert!(detail.starts_with("person.create:"), "{detail}");

                let roles = roles_of(&root).await;
                let detail =
                    expect_problem(roles.list_request().send().promise.await, NOT_POSSIBLE);
                assert!(detail.starts_with("role.read:"), "{detail}");

                let mut subscribe = root.subscribe_request();
                let seen: fs_interfaces::common_capnp::change_listener::Client =
                    capnp_rpc::new_client(Never);
                subscribe.get().set_listener(seen);
                let detail = expect_problem(subscribe.send().promise.await, NOT_POSSIBLE);
                assert!(detail.starts_with("person.subscribe:"), "{detail}");
            })
            .await;
        })
        .await;
}

struct Never;
impl fs_interfaces::common_capnp::change_listener::Server for Never {}
