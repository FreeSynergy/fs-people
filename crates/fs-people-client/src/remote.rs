//! What the client takes from the root of the people contract.
//!
//! Pattern: Strategy — the one part of the connection that is the people program's own: the
//! handshake and the two capabilities a call may use. The thread, the queue and the deadlines are
//! the shared `Worker` of `fs-interfaces`.

use fs_interfaces::Capabilities;
use fs_interfaces::people_capnp::{people, persons, roles};
use fs_interfaces::rpc::client::Remote as Contract;

/// The capabilities of the server a call may use.
#[derive(Clone)]
pub(crate) struct Remote {
    pub(crate) persons: persons::Client,
    pub(crate) roles: roles::Client,
}

impl Contract for Remote {
    type Root = people::Client;

    async fn open(root: people::Client) -> Result<(Self, Capabilities), capnp::Error> {
        let report = root.support_report_request().send().promise.await?;
        let capabilities = Capabilities::read(report.get()?.get_report()?)?;
        let persons = root.persons_request().send().promise.await?;
        let persons = persons.get()?.get_persons()?;
        let roles = root.roles_request().send().promise.await?;
        let roles = roles.get()?.get_roles()?;
        Ok((Self { persons, roles }, capabilities))
    }
}
