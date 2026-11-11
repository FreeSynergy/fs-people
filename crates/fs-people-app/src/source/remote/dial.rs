//! How the connection to the people server is made.
//!
//! Pattern: Strategy (`Dial`: plain or with the version handshake of fs-interfaces).

use std::path::PathBuf;
use std::sync::Arc;

use fs_interfaces::people_capnp::people;
use fs_interfaces::rpc::{Connection, Expected, ServerStarter, connect, connect_matching};

/// Plain: a server that is there (what the tests have). Matching: the version handshake, a server of another version
/// is replaced and none running is started by the `starter`.
#[derive(Clone)]
pub(super) enum Dial {
    Plain(PathBuf),
    Matching {
        socket: PathBuf,
        expected: Expected,
        starter: Arc<dyn ServerStarter + Send + Sync>,
    },
}

impl Dial {
    /// Connect; the error is the text for the person.
    pub(super) async fn connect(&self) -> Result<Connection<people::Client>, String> {
        match self {
            Self::Plain(socket) => connect::<people::Client>(socket).await.map_err(|e| {
                format!(
                    "no people server on {}: {e}; start it with `fs-people serve`",
                    socket.display()
                )
            }),
            Self::Matching {
                socket,
                expected,
                starter,
            } => connect_matching::<people::Client>(socket, expected, starter.as_ref())
                .await
                .map_err(|e| e.to_string()),
        }
    }
}
