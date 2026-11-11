//! A server that passes the handshake and then says nothing: the window must not wait for ever.
//!
//! Pattern: none (tests) — a fake server (`people::Server`) answers `Control.version` (through `run_until_signal_as`) and
//! then keeps silent, once on the first questions after the handshake, once on a call from the window. Own file: the
//! servers are never stopped, they end with the process.

use std::future::Future;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use capnp_rpc::new_client;
use fs_interfaces::people_capnp::{people, persons, roles};
use fs_interfaces::rpc::{ServerIdentity, run_until_signal_as};
use fs_people_app::{Limits, RemotePeople};
use fs_people_core::{Error, ListFilter, People};

const LIMIT: Duration = Duration::from_millis(400);

fn limits() -> Limits {
    Limits {
        start: LIMIT,
        call: LIMIT,
    }
}

/// A capability that knows no method, except `persons.list`, which never answers.
struct Quiet;
impl roles::Server for Quiet {}
impl persons::Server for Quiet {
    async fn list(
        self: Rc<Self>,
        _: persons::ListParams,
        _: persons::ListResults,
    ) -> Result<(), capnp::Error> {
        std::future::pending().await
    }
}

/// The root: with `hang_start` the first accessor of the start (`persons`) never answers.
struct Root {
    hang_start: bool,
}
impl people::Server for Root {
    async fn persons(
        self: Rc<Self>,
        _: people::PersonsParams,
        mut r: people::PersonsResults,
    ) -> Result<(), capnp::Error> {
        if self.hang_start {
            std::future::pending::<()>().await;
        }
        r.get().set_persons(new_client(Quiet));
        Ok(())
    }
    fn roles(
        self: Rc<Self>,
        _: people::RolesParams,
        mut r: people::RolesResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> {
        r.get().set_roles(new_client(Quiet));
        std::future::ready(Ok(()))
    }
}

fn serve(socket: &Path, hang_start: bool) {
    let socket = socket.to_path_buf();
    let (up, started) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let identity = ServerIdentity::new("fs-people", env!("CARGO_PKG_VERSION"));
        let _ = run_until_signal_as(&socket, identity, || async move {
            up.send(()).ok();
            let root: people::Client = new_client(Root { hang_start });
            Ok(root.client)
        });
    });
    started.recv().unwrap();
}

fn remote(socket: &Path) -> RemotePeople {
    let refuse = || -> std::io::Result<()> { Err(std::io::Error::other("must not be started")) };
    RemotePeople::matching_with(socket, Arc::new(refuse), limits())
}

fn unreachable_text(error: Error) -> String {
    match error {
        Error::Unreachable(text) => text,
        other => panic!("expected a visible error, got {other:?}"),
    }
}

#[test]
fn a_server_that_is_silent_after_the_handshake_does_not_keep_the_window_from_opening() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("people.sock");
    serve(&socket, true);
    let began = Instant::now();
    let remote = remote(&socket);
    assert!(
        began.elapsed() < LIMIT * 10,
        "the start waited {:?}",
        began.elapsed()
    );
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let error = runtime
        .block_on(remote.list(&ListFilter::default()))
        .unwrap_err();
    assert!(
        unreachable_text(error).contains("did not answer"),
        "the reason of the start is kept"
    );
}

#[test]
fn a_call_that_the_server_never_answers_ends_with_the_limit_and_the_connection_stays_usable() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("people.sock");
    serve(&socket, false);
    let remote = remote(&socket);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for _ in 0..2 {
        let began = Instant::now();
        let error = runtime
            .block_on(remote.list(&ListFilter::default()))
            .unwrap_err();
        let took = began.elapsed();
        assert!(
            unreachable_text(error).contains("did not answer"),
            "no visible reason"
        );
        assert!(took >= LIMIT, "too early: {took:?}");
        assert!(took < LIMIT * 5, "the limit did not hold: {took:?}");
    }
}
