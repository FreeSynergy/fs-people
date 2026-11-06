//! `People` root interface: support report, persons, roles, subscribe.
//!
//! Pattern: Adapter + Factory — the root hands out the `Persons` and `Roles` capabilities.
//! [`PeopleServer`] is what a program serves; it depends on the three traits, never on `PeopleRepo`
//! (ADR 0043).

use std::rc::Rc;
use std::sync::Arc;

use capnp_rpc::new_client;
use fs_interfaces::people_capnp::people;
use fs_people_core::{People, PeopleFeed, RoleBook};

use crate::feed::subscribe;
use crate::persons::PersonsImpl;
use crate::roles_service::RolesImpl;

/// The version of `people.capnp` this server answers.
pub const CONTRACT_VERSION: &str = "people 0.1 (draft)";

/// The root capability of the people program.
pub struct PeopleServer {
    people: Arc<dyn People>,
    roles: Arc<dyn RoleBook>,
    feed: Arc<dyn PeopleFeed>,
}

impl PeopleServer {
    /// Serve this provider.
    pub fn new(
        people: Arc<dyn People>,
        roles: Arc<dyn RoleBook>,
        feed: Arc<dyn PeopleFeed>,
    ) -> Self {
        Self {
            people,
            roles,
            feed,
        }
    }

    /// The capability to hand to `fs_interfaces::rpc::serve`.
    #[must_use]
    pub fn into_client(self) -> people::Client {
        new_client(self)
    }
}

type Done = Result<(), capnp::Error>;

impl people::Server for PeopleServer {
    async fn support_report(
        self: Rc<Self>,
        _: people::SupportReportParams,
        mut results: people::SupportReportResults,
    ) -> Done {
        let report = self.people.capabilities();
        report.write(CONTRACT_VERSION, results.get().init_report());
        Ok(())
    }

    async fn persons(
        self: Rc<Self>,
        _: people::PersonsParams,
        mut results: people::PersonsResults,
    ) -> Done {
        let persons = PersonsImpl {
            people: self.people.clone(),
        };
        results.get().set_persons(new_client(persons));
        Ok(())
    }

    async fn roles(
        self: Rc<Self>,
        _: people::RolesParams,
        mut results: people::RolesResults,
    ) -> Done {
        let roles = RolesImpl {
            roles: self.roles.clone(),
        };
        results.get().set_roles(new_client(roles));
        Ok(())
    }

    async fn subscribe(
        self: Rc<Self>,
        params: people::SubscribeParams,
        mut results: people::SubscribeResults,
    ) -> Done {
        let listener = params.get()?.get_listener()?;
        let subscription = subscribe(&self.feed, self.people.clone(), listener)?;
        results.get().set_subscription(subscription);
        Ok(())
    }
}
