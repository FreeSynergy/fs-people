//! How long the client waits for a server that says nothing.
//!
//! Pattern: Value object — two fixed deadlines with defaults; a caller (or a test) may choose
//! others with `RpcPeople::connect_with`.

use std::time::Duration;

/// How long `connect` waits for the server to answer the handshake.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// How long one call waits for its answer. A write that ran out of time may still have happened:
/// the server finishes a write also if the caller goes away.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// The deadlines of one connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeouts {
    /// For the handshake of `connect`.
    pub connect: Duration,
    /// For each call after it.
    pub call: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: CONNECT_TIMEOUT,
            call: CALL_TIMEOUT,
        }
    }
}
