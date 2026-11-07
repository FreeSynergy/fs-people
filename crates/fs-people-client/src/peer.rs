//! The connection: what a frontend holds.
//!
//! Pattern: Adapter — `People` and `RoleBook` of the core in front of the Cap'n Proto client of
//! `people.capnp`. The thread of the connection is in `worker.rs`; the calls in `calls/`; the traits
//! in `people_impl.rs` and `roles_impl.rs`. Interface first (ADR 0043): `connect` was written down
//! before the calls.

use std::future::Future;
use std::io;
use std::path::Path;
use std::time::Duration;

use fs_interfaces::Capabilities;
use fs_people_core::{Error, Result};
use fs_store::Error as StoreError;
use tokio::sync::oneshot;

use crate::reply::Reply;
use crate::timeouts::Timeouts;
use crate::worker::{Job, Remote, Worker, start};

fn silent(limit: Duration) -> Error {
    Error::Store(StoreError::Backend(format!(
        "people server did not answer within {limit:?}"
    )))
}

/// The people program as seen over its socket. Implements `People` and `RoleBook`.
///
/// Calls from any thread are fine; they queue for the one thread that owns the connection and
/// overlap there. Dropping it closes the connection.
pub struct RpcPeople {
    worker: Worker,
    capabilities: Capabilities,
    timeouts: Timeouts,
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
        let (worker, capabilities) = start(socket.to_path_buf(), timeouts.connect)?;
        Ok(Self {
            worker,
            capabilities,
            timeouts,
        })
    }

    /// What the server reported when this connection was made. A snapshot: the server is not asked
    /// again.
    pub(crate) fn report(&self) -> Capabilities {
        self.capabilities.clone()
    }

    /// Run `work` on the connection's thread and wait for its answer.
    pub(crate) async fn call<T, F, Fut>(&self, work: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(Remote) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T>> + 'static,
    {
        let gone = || {
            Error::Store(StoreError::Backend(
                "people server: the connection is gone".into(),
            ))
        };
        let limit = self.timeouts.call;
        let (reply, answer) = oneshot::channel();
        let job: Job = Box::new(move |remote| {
            Box::pin(async move {
                let reply = Reply::started(reply);
                // The timer is the connection thread's, so the caller's runtime needs no clock.
                let done = tokio::time::timeout(limit, work(remote)).await;
                reply.send(done.unwrap_or_else(|_| Err(silent(limit))));
            })
        });
        self.worker.jobs.send(job).map_err(|_| gone())?;
        answer.await.map_err(|_| gone())?
    }
}
