//! `serve`: the server as a command of the one program (ADR 0063).
//!
//! Pattern: Facade — reads its locations, opens the store, serves until a signal. The binary
//! `fs-people` calls it for `fs-people serve`.

use std::ffi::OsString;
use std::sync::Arc;

use fs_interfaces::rpc::{
    ServerIdentity, ServerOptions, prepare_data_dir, run_until_signal_as, stop_self,
};
use fs_objects::{Store, TursoStore};
use fs_people_core::PeopleRepo;

use crate::PeopleServer;

/// The help text of `fs-people serve`.
pub const HELP: &str = "fs-people serve [--socket PATH] [--data DIR] [--actor IRI] [--lease FILE]\n--lease FILE: end when no window holds this lock any more (started by a window, not by hand)\nDefaults: $XDG_RUNTIME_DIR/freesynergy/people.sock, $XDG_DATA_HOME/freesynergy/people, the local owner";

/// Why `serve` ended without serving.
#[derive(Debug, PartialEq, Eq)]
pub enum ServeError {
    /// The words were wrong; the caller should exit with 2.
    Usage(String),
    /// The server could not start or stopped with an error; exit with 1.
    Failed(String),
}

/// Serve `people.capnp` until a signal arrives. `args` are the words after `serve`.
///
/// # Errors
/// [`ServeError::Usage`] for unknown or incomplete words, [`ServeError::Failed`] for everything else.
pub fn serve(args: impl IntoIterator<Item = OsString>) -> Result<(), ServeError> {
    let (lease, args) =
        crate::lease::take_lease(args.into_iter().collect()).map_err(ServeError::Usage)?;
    let Some(options) = ServerOptions::parse("people", args).map_err(ServeError::Usage)? else {
        println!("{HELP}");
        return Ok(());
    };
    let ServerOptions {
        socket,
        data,
        actor,
        instance,
        facet,
    } = options;
    // The server serves everything; the facet is the window's view. A name this program does not have is still refused,
    // so a wrong link is found at the start.
    if let Some(facet) = facet {
        fs_people_core::check_facet(&facet).map_err(ServeError::Usage)?;
    }
    // Name and version go to `Control.version`: a window of another version replaces this server (fs-interfaces handshake).
    let identity = ServerIdentity::new("fs-people", env!("CARGO_PKG_VERSION"));
    run_until_signal_as(&socket, identity, || async move {
        // The watcher ends this process by SIGTERM, so it starts only here, where the stop signal is already registered
        // (`run_until_signal` registers it before it binds): a look that finds nobody early can no longer kill the
        // process by the signal.
        if let Some(lease) = lease {
            // The window that started this server holds the lease; when none does, end as a signal would.
            crate::lease::watch(lease, stop_self);
        }
        prepare_data_dir(&data)?;
        let store = TursoStore::open(data.join("objects.db"))
            .await
            .map_err(std::io::Error::other)?;
        let store: Arc<dyn Store> = Arc::new(store);
        let repo = PeopleRepo::new(store, actor);
        let repo = Arc::new(match instance {
            Some(domain) => repo.with_instance(domain),
            None => repo,
        });
        let restored = repo
            .restore_role_config()
            .await
            .map_err(std::io::Error::other)?;
        for line in restored.skipped() {
            eprintln!("fs-people: skipped stored role file: {line}");
        }
        let server = PeopleServer::new(repo.clone(), repo.clone(), repo);
        Ok(server.into_client().client)
    })
    .map_err(|e| ServeError::Failed(e.to_string()))
}
