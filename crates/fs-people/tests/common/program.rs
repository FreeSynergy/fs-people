//! The real binary in a test.
//!
//! Pattern: RAII — a server goes when the test does.

#![allow(dead_code)] // each test file uses a different subset

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// A server that goes when the test does, also when it fails: one left behind keeps a piped test run
/// from ever ending.
pub struct Server(pub Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn start(socket: &Path, data: &Path) -> Server {
    start_with(socket, data, &[])
}

pub fn start_with(socket: &Path, data: &Path, more: &[&str]) -> Server {
    Server(
        Command::new(env!("CARGO_BIN_EXE_fs-people"))
            .arg("serve")
            .arg("--socket")
            .arg(socket)
            .arg("--data")
            .arg(data)
            .args(more)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

pub async fn wait_for(socket: &Path) {
    for _ in 0..200 {
        if tokio::net::UnixStream::connect(socket).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the server did not come up");
}

/// The server answers a call. `wait_for` only sees the socket, which is bound before the store is opened and before the
/// program listens for its stop signal; a TERM sent in between ends it by the signal (no exit code) instead of by the
/// orderly stop. An answered call means both are done. Must run on a `LocalSet`.
pub async fn wait_ready(socket: &Path) {
    use fs_interfaces::people_capnp::people;
    for _ in 0..1200 {
        if let Ok(c) = fs_interfaces::rpc::connect::<people::Client>(socket).await
            && c.client
                .support_report_request()
                .send()
                .promise
                .await
                .is_ok()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the server did not answer");
}

pub fn terminate(mut server: Server) {
    let child = &mut server.0;
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().to_string())
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(child.wait().unwrap().code(), Some(0));
}
