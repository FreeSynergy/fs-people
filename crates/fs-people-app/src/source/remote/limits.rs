//! The time limits of the calls to the people server.
//!
//! Pattern: none — plain configuration data (`Limits`) and the words of a timeout.

use std::future::Future;
use std::time::Duration;

use fs_interfaces::rpc::client::{CALL_TIMEOUT, Fault, call_limited};
use fs_people_core::{Error, Result};

/// The name of the server in the words of a timeout.
pub(super) const SERVER: &str = "people";

/// The words of a connection that ended under a call; the proxy connects again at the next call.
pub(super) const LOST: &str = "the people server went away; asking again";

/// What the caller waits on top of a limit for a worker that cannot run the job.
pub(super) const GRACE: Duration = Duration::from_secs(2);

/// How long the window waits for the server before it says so (no call waits for ever; a window never freezes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The first questions right after the handshake: the window is not open yet, so short.
    pub start: Duration,
    /// Every other call: the default of fs-interfaces (`CALL_TIMEOUT`, 30 s).
    pub call: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            start: Duration::from_secs(10),
            call: CALL_TIMEOUT,
        }
    }
}

/// A call that ran out of time, as the person sees it.
pub(super) fn timed_out(fault: &Fault) -> Error {
    Error::Unreachable(fault.to_string())
}

/// Run `call` for at most `limit`; a server that does not answer is a visible error, the connection stays usable.
pub(super) async fn within<T>(limit: Duration, call: impl Future<Output = Result<T>>) -> Result<T> {
    call_limited(SERVER, limit, call)
        .await
        .unwrap_or_else(|fault| Err(timed_out(&fault)))
}
