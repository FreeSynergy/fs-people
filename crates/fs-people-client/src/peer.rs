//! The connection: what a frontend holds.
//!
//! Pattern: Adapter — `People` and `RoleBook` of the core in front of the Cap'n Proto client of
//! `people.capnp`. The thread of the connection is the shared `Worker` of `fs-interfaces`, with the contract in `remote.rs`; the calls in `calls/`; the traits
//! in `people_impl.rs` and `roles_impl.rs`. Interface first (ADR 0043): `connect` was written down
//! before the calls.

use std::future::Future;
use std::io;
use std::path::Path;

use fs_interfaces::Capabilities;
use fs_interfaces::rpc::client::{Fault, Timeouts, Worker};
use fs_people_core::{Error, Result};
use fs_objects::Error as StoreError;

use crate::offered::offered;
use crate::remote::Remote;

/// The name of the server in every error text (`people server: ...`).
const SERVER: &str = "people";

/// The people program as seen over its socket. Implements `People` and `RoleBook`.
///
/// Calls from any thread are fine; they queue for the one thread that owns the connection and
/// overlap there. Dropping it closes the connection.
pub struct RpcPeople {
    worker: Worker<Remote>,
    capabilities: Capabilities,
}

impl RpcPeople {
    /// Connect to the `fs-people-server` that listens on `socket` and ask what it can do.
    ///
    /// Blocks the calling thread until the server has answered; call it before the window opens, not
    /// from inside an async task of a single-threaded runtime.
    ///
    /// # Errors
    /// Nothing listens there, the server runs as another user, or it does not answer the contract.
    pub fn connect(socket: &Path) -> io::Result<Self> {
        Self::connect_with(socket, Timeouts::default())
    }

    /// [`connect`](Self::connect) with other deadlines than [`CONNECT_TIMEOUT`](crate::CONNECT_TIMEOUT)
    /// and [`CALL_TIMEOUT`](crate::CALL_TIMEOUT). A server that stays silent ends the wait with an
    /// error that names the deadline.
    ///
    /// # Errors
    /// As for [`connect`](Self::connect), and the server did not answer in time.
    pub fn connect_with(socket: &Path, timeouts: Timeouts) -> io::Result<Self> {
        let (worker, capabilities) = Worker::start(SERVER, socket, timeouts)?;
        Ok(Self {
            worker,
            capabilities: offered(&capabilities),
        })
    }

    /// What the server reported when this connection was made. A snapshot: the server is not asked
    /// again.
    pub(crate) fn report(&self) -> Capabilities {
        self.capabilities.clone()
    }

    /// Run `work` on the connection's thread and wait for its answer. A call that got none (the
    /// connection is gone, the job ended, the deadline ran out) is a `Backend` error with the text of
    /// the `Fault`: the core has no variant for it.
    pub(crate) async fn call<T, F, Fut>(&self, work: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(Remote) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + 'static,
    {
        let fault = |f: Fault| Error::Store(StoreError::Backend(f.to_string()));
        self.worker.call(work).await.map_err(fault)?
    }
}
