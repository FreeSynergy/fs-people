//! The version handshake of the window: a server of this version is used as it is, and when none fits (and none can be
//! started) the person gets a visible reason and the next call asks again, never a black window.
//!
//! Pattern: none (tests). Nothing here stops a server: `Control.stop` is one flag per process.

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use fs_interfaces::people_capnp::people;
use fs_interfaces::rpc::{ServerIdentity, run_until_signal_as};
use fs_objects::{Store, TursoStore};
use fs_people_app::RemotePeople;
use fs_people_core::{Error, ListFilter, People, PeopleRepo};
use fs_people_server::PeopleServer;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A real server that says it is `version`; it ends with the process.
fn serve_as(socket: &Path, version: &str) {
    let (socket, version) = (socket.to_path_buf(), version.to_owned());
    let (up, started) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let identity = ServerIdentity::new("fs-people", version);
        let _ = run_until_signal_as(&socket, identity, || async move {
            let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
            let repo = Arc::new(PeopleRepo::new(store, "https://local.invalid/users/owner"));
            let root: people::Client =
                PeopleServer::new(repo.clone(), repo.clone(), repo).into_client();
            up.send(()).ok();
            Ok(root.client)
        });
    });
    started.recv().unwrap();
}

#[test]
fn a_server_of_this_version_is_used_as_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("people.sock");
    serve_as(&socket, VERSION);
    let refuse = || -> io::Result<()> { Err(io::Error::other("must not be started")) };
    let remote = RemotePeople::matching(&socket, Arc::new(refuse));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let found = runtime
        .block_on(remote.list(&ListFilter::default()))
        .unwrap();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn no_server_and_no_start_is_a_visible_reason_that_the_next_call_can_end() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("people.sock");
    let tried = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&tried);
    let refuse = move || -> io::Result<()> {
        flag.store(true, Ordering::SeqCst);
        Err(io::Error::other("no executable"))
    };
    let remote = RemotePeople::matching(&socket, Arc::new(refuse));
    assert!(tried.load(Ordering::SeqCst), "the starter was asked");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    match runtime.block_on(remote.list(&ListFilter::default())) {
        Err(Error::Unreachable(text)) => assert!(text.contains("no executable"), "{text}"),
        other => panic!("expected a visible reason, got {other:?}"),
    }
    assert!(remote.reconnect().is_err());
    // The person fixes it (here: the server is there now); the next call connects again.
    serve_as(&socket, VERSION);
    let found = runtime
        .block_on(remote.list(&ListFilter::default()))
        .unwrap();
    assert!(found.is_empty(), "{found:?}");
}
