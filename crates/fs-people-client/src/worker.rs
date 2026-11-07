//! The thread that owns the Cap'n Proto connection.
//!
//! Pattern: Command queue (active object) — a capnp-rpc client is `!Send`, so it lives on one thread
//! with a current-thread runtime and a `LocalSet`. Callers on any thread send a [`Job`] (a closure
//! that gets the capabilities and answers through its own channel); the thread runs each as a local
//! task, so calls overlap. Dropping the [`Worker`] ends the loop, closes the connection and joins.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::mpsc;
use std::thread::JoinHandle;

use fs_interfaces::Capabilities;
use fs_interfaces::people_capnp::{people, persons, roles};
use fs_interfaces::rpc::connect;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

/// The capabilities of the server a job may call.
#[derive(Clone)]
pub(crate) struct Remote {
    pub(crate) persons: persons::Client,
    pub(crate) roles: roles::Client,
}

/// A piece of work for the connection's thread.
pub(crate) type Job = Box<dyn FnOnce(Remote) -> Pin<Box<dyn Future<Output = ()>>> + Send>;

/// The thread, and the way to reach it.
pub(crate) struct Worker {
    pub(crate) jobs: UnboundedSender<Job>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        // The sender goes with `self.jobs` after this; close the queue first by swapping it out.
        let (closed, _) = unbounded_channel();
        drop(std::mem::replace(&mut self.jobs, closed));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn other(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(format!("people server: {e}"))
}

/// What the thread learned on connecting.
async fn handshake(socket: &Path) -> io::Result<(Remote, Capabilities, impl Sized)> {
    let connection = connect::<people::Client>(socket).await?;
    let root = connection.client.clone();
    let report = root
        .support_report_request()
        .send()
        .promise
        .await
        .map_err(other)?;
    let capabilities =
        Capabilities::read(report.get().and_then(|r| r.get_report()).map_err(other)?)
            .map_err(other)?;
    let persons = root.persons_request().send().promise.await.map_err(other)?;
    let persons = persons.get().and_then(|r| r.get_persons()).map_err(other)?;
    let roles = root.roles_request().send().promise.await.map_err(other)?;
    let roles = roles.get().and_then(|r| r.get_roles()).map_err(other)?;
    Ok((Remote { persons, roles }, capabilities, connection))
}

/// Start the thread and wait until it is connected.
pub(crate) fn start(socket: PathBuf) -> io::Result<(Worker, Capabilities)> {
    let (jobs, mut queue) = unbounded_channel::<Job>();
    let (ready, answer) = mpsc::channel::<io::Result<Capabilities>>();
    let thread = std::thread::Builder::new()
        .name("fs-people-client".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(e) => return drop(ready.send(Err(e))),
            };
            let local = tokio::task::LocalSet::new();
            local.block_on(&runtime, async {
                let (remote, capabilities, connection) = match handshake(&socket).await {
                    Ok(done) => done,
                    Err(e) => return drop(ready.send(Err(e))),
                };
                let _ = ready.send(Ok(capabilities));
                while let Some(job) = queue.recv().await {
                    tokio::task::spawn_local(job(remote.clone()));
                }
                drop(remote);
                drop(connection);
            });
            // Let the disconnect and the jobs still under way finish.
            runtime.block_on(local);
        })?;
    let capabilities = answer
        .recv()
        .map_err(|_| other("the connection thread ended"))??;
    Ok((
        Worker {
            jobs,
            thread: Some(thread),
        },
        capabilities,
    ))
}
