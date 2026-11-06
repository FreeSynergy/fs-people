//! The provider rule (ADR 0045): every provider has every method; what it does not offer answers
//! an explicit "not possible", and its report says the same.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use common::repo;
use fs_interfaces::Capabilities;
use fs_people_core::{Card, Error, ListFilter, People, Result, RoleBook};
use fs_store::{ObjectId, StoredObject};

/// A read-only provider, for example an address book it cannot write to.
struct ReadOnly;

#[async_trait]
impl People for ReadOnly {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new("read-only");
        c.supported("person.read");
        c.unsupported("person.create", "this source cannot be written");
        c
    }
    async fn get(&self, _: ObjectId) -> Result<Option<StoredObject>> {
        Ok(None)
    }
    async fn list(&self, _: &ListFilter) -> Result<Vec<StoredObject>> {
        Ok(vec![])
    }
}

#[async_trait]
impl RoleBook for ReadOnly {}

#[tokio::test]
async fn a_missing_method_says_not_possible() {
    let p: Arc<dyn People> = Arc::new(ReadOnly);
    assert!(p.list(&ListFilter::default()).await.unwrap().is_empty());
    let Err(Error::Unsupported(u)) = p.create(Card::individual("Ada")).await else {
        panic!("expected an explicit not possible");
    };
    assert_eq!(u.capability(), "person.create");
    assert_eq!(u.status(), 501);
    assert!(!p.capabilities().is_supported("person.create"));

    let roles: Arc<dyn RoleBook> = Arc::new(ReadOnly);
    assert!(matches!(roles.roles().await, Err(Error::Unsupported(_))));
    assert!(matches!(
        roles.is_ai(ObjectId::new()).await,
        Err(Error::Unsupported(_))
    ));
}

#[tokio::test]
async fn the_store_backed_provider_reports_what_it_can() {
    let p: Arc<dyn People> = Arc::new(repo().await);
    let caps = p.capabilities();
    for id in [
        "person.create",
        "relation.write",
        "role.assign",
        "ai.mark",
        "same.link",
    ] {
        assert!(caps.is_supported(id), "{id}");
    }
    assert!(caps.why_not("person.subscribe").is_some());
}

#[tokio::test]
async fn the_shipped_roles_load() {
    let r = repo().await;
    let ids: Vec<_> = r.roles().await.unwrap().into_iter().map(|d| d.id).collect();
    assert_eq!(ids, ["ai", "customer", "member", "user"]);
}
