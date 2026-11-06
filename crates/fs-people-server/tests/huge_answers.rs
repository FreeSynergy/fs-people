//! Answers without a cursor that would not fit: refused in words at each method. A provider that holds
//! this much takes minutes to build from cards, so a small one answers here; the real one is `large.rs`.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use common::{TOO_LARGE, expect_problem, persons_of, roles_of, unknown_id, with_root};
use fs_interfaces::Capabilities;
use fs_interfaces::rpc::{MAX_PAGE_BYTES, write_uuid};
use fs_people_core::{
    Exporter, ListFilter, MatchKind, MatchReason, People, PeopleFeed, Result as CoreResult,
    RoleAssignment, RoleBook, RoleDefinition, RoleScope, SameAsProposal,
};
use fs_people_server::PeopleServer;
use fs_store::{ObjectId, StoredObject};
use serde_json::{Map, json};

const MANY: usize = 2_000_000;
const MEGABYTE: usize = 1024 * 1024;

struct Huge;

fn ids() -> Vec<ObjectId> {
    vec![ObjectId::new(); MANY]
}

#[async_trait]
impl People for Huge {
    fn capabilities(&self) -> Capabilities {
        Capabilities::new("huge")
    }
    async fn get(&self, _: ObjectId) -> CoreResult<Option<StoredObject>> {
        Ok(None)
    }
    async fn list(&self, _: &ListFilter) -> CoreResult<Vec<StoredObject>> {
        Ok(vec![])
    }
    async fn linked_to(&self, _: ObjectId) -> CoreResult<Vec<ObjectId>> {
        Ok(ids())
    }
    async fn relations(&self, _: ObjectId) -> CoreResult<Vec<(String, String)>> {
        Ok(vec![("u".into(), "r".into()); MANY])
    }
    async fn find_same_person_proposals(&self) -> CoreResult<Vec<SameAsProposal>> {
        let id = ObjectId::new();
        let reasons = vec![MatchReason {
            kind: MatchKind::Email,
            value: "a@x.org".into(),
        }];
        Ok(vec![
            SameAsProposal {
                a: id,
                b: id,
                reasons
            };
            MANY
        ])
    }
    async fn export(&self, _: &dyn Exporter, _: ObjectId) -> CoreResult<String> {
        Ok("x".repeat(MAX_PAGE_BYTES + 1))
    }
}

#[async_trait]
impl RoleBook for Huge {
    async fn roles(&self) -> CoreResult<Vec<RoleDefinition>> {
        let mut one = RoleDefinition::new("r", "R", "o", RoleScope::Public);
        one.description.insert("en".into(), "x".repeat(MEGABYTE));
        Ok(vec![one; 40])
    }
    async fn roles_of(&self, person: ObjectId) -> CoreResult<Vec<RoleAssignment>> {
        let mut details = Map::new();
        details.insert("a".into(), json!("x".repeat(MEGABYTE)));
        let one = RoleAssignment {
            id: person,
            person,
            role: "r".into(),
            scope: None,
            details,
        };
        Ok(vec![one; 40])
    }
    async fn people_with_personal_role(&self, _: &str) -> CoreResult<Vec<ObjectId>> {
        Ok(ids())
    }
    async fn people_with_role(&self, _: &str, _: Option<&str>) -> CoreResult<Vec<ObjectId>> {
        Ok(ids())
    }
}

impl PeopleFeed for Huge {}

#[tokio::test(flavor = "current_thread")]
async fn every_list_without_a_cursor_refuses_what_does_not_fit() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let huge = Arc::new(Huge);
            let server = PeopleServer::new(huge.clone(), huge.clone(), huge);
            with_root(server.into_client(), |root| async move {
                let persons = persons_of(&root).await;
                let roles = roles_of(&root).await;
                let id = unknown_id();

                let mut linked = persons.linked_to_request();
                write_uuid(linked.get().init_id(), Some(id));
                expect_problem(linked.send().promise.await, TOO_LARGE);

                let mut relations = persons.relations_request();
                write_uuid(relations.get().init_id(), Some(id));
                expect_problem(relations.send().promise.await, TOO_LARGE);

                let proposals = persons.proposals_request();
                expect_problem(proposals.send().promise.await, TOO_LARGE);

                let mut export = persons.export_text_request();
                write_uuid(export.get().init_id(), Some(id));
                export.get().set_media_type("text/vcard");
                expect_problem(export.send().promise.await, TOO_LARGE);

                expect_problem(roles.list_request().send().promise.await, TOO_LARGE);

                let mut held = roles.roles_of_request();
                write_uuid(held.get().init_person(), Some(id));
                expect_problem(held.send().promise.await, TOO_LARGE);

                let mut with = roles.people_with_request();
                with.get().set_role("r");
                expect_problem(with.send().promise.await, TOO_LARGE);
                let mut with = roles.people_with_request();
                with.get().set_role("r");
                with.get().set_scope("https://x/g");
                expect_problem(with.send().promise.await, TOO_LARGE);

                // Nothing of this took the connection along.
                let answer = root.support_report_request().send().promise.await.unwrap();
                let provider = answer
                    .get()
                    .unwrap()
                    .get_report()
                    .unwrap()
                    .get_provider()
                    .unwrap();
                assert_eq!(provider.to_str().unwrap(), "huge");
            })
            .await;
        })
        .await;
}
