//! The connection: what a frontend holds.
//!
//! Pattern: Adapter — the traits of the core in front of the Cap'n Proto client. Written down as
//! the interface first (ADR 0043); the calls follow once the tests exist.

use std::io;
use std::path::Path;

/// The people program as seen over its socket. Implements `People` and `RoleBook`.
///
/// Dropping it closes the connection.
#[derive(Debug)]
pub struct RpcPeople {
    _private: (),
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
        Err(io::Error::other(format!(
            "not written yet: {}",
            socket.display()
        )))
    }
}
