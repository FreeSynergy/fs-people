//! `RemotePeople`: `People` over the Cap'n Proto socket of the people server (ADR 0063).
//!
//! Pattern: Proxy — the same contract as the in-process core, answered by the server. A Cap'n Proto
//! client lives on one thread, so a worker thread owns the connection and the proxy hands it calls
//! through a channel; the window thread waits for the answer like for the in-process core.
//!
//! Provisional: only what the window needs (read, list, search, create, update, delete). Roles are
//! not asked, so the window shows none over the socket (`RoleBook` is not implemented here).

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_interfaces::rpc::client::Fault;
use fs_interfaces::rpc::{Expected, ServerStarter};
use fs_objects::{ObjectId, StoredObject};
use fs_people_core::{
    Card, Error, ListFilter, People, Result, Revision, RoleAssignment, RoleBook, RoleDefinition,
};
use tokio::sync::{mpsc, oneshot};

use super::call::{Call, Reply, Revisions};
use super::dial::Dial;
pub use super::limits::Limits;
use super::limits::{GRACE, LOST, SERVER, timed_out};
use super::worker::worker;

/// The state of the connection: a worker that answers, or why there is none (the window shows it).
enum Link {
    Up(mpsc::UnboundedSender<Call>),
    Down(String),
}

/// The people server behind a socket, as a provider of [`People`].
pub struct RemotePeople {
    dial: Dial,
    limits: Limits,
    link: Mutex<Link>,
    revisions: Revisions,
}

impl RemotePeople {
    /// Connect to the server that is on `socket`, with no version check and no start (what the tests need).
    ///
    /// # Errors
    /// The text for the person when nothing answers on the socket.
    pub fn connect(socket: &Path) -> std::result::Result<Self, String> {
        Self::connect_with(socket, Limits::default())
    }

    /// [`RemotePeople::connect`] with other [`Limits`].
    ///
    /// # Errors
    /// The text for the person when nothing answers on the socket.
    pub fn connect_with(socket: &Path, limits: Limits) -> std::result::Result<Self, String> {
        let dial = Dial::Plain(socket.to_path_buf());
        let revisions = Revisions::default();
        let calls = open(&dial, limits, &revisions)?;
        Ok(Self {
            dial,
            limits,
            link: Mutex::new(Link::Up(calls)),
            revisions,
        })
    }

    /// Reach the server on `socket` that fits this program: the version handshake of fs-interfaces. One that runs and
    /// differs is replaced, none running is started by `starter`. This never fails: when no fitting server can be
    /// reached, the remote keeps the reason and every call answers with it, and the next call asks again
    /// ([`RemotePeople::reconnect`]). The window opens in any case.
    pub fn matching(socket: &Path, starter: Arc<dyn ServerStarter + Send + Sync>) -> Self {
        Self::matching_with(socket, starter, Limits::default())
    }

    /// [`RemotePeople::matching`] with other [`Limits`]. A server that passes the handshake and then does not answer
    /// the first questions is reported after [`Limits::start`], not waited for.
    pub fn matching_with(
        socket: &Path,
        starter: Arc<dyn ServerStarter + Send + Sync>,
        limits: Limits,
    ) -> Self {
        let dial = Dial::Matching {
            socket: socket.to_path_buf(),
            // The program's name too: a wrong `--socket` (the server of another program) is left alone, never replaced.
            expected: Expected::new(env!("CARGO_PKG_VERSION")).with_program("fs-people"),
            starter,
        };
        let revisions = Revisions::default();
        let link = match open(&dial, limits, &revisions) {
            Ok(calls) => Link::Up(calls),
            Err(reason) => Link::Down(reason),
        };
        Self {
            dial,
            limits,
            link: Mutex::new(link),
            revisions,
        }
    }

    /// Connect again (after a failed handshake, or a server that went away) and keep the new connection.
    ///
    /// # Errors
    /// The text for the person.
    pub fn reconnect(&self) -> std::result::Result<(), String> {
        let opened = open(&self.dial, self.limits, &self.revisions);
        let mut link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        match opened {
            Ok(calls) => {
                *link = Link::Up(calls);
                Ok(())
            }
            Err(reason) => {
                *link = Link::Down(reason.clone());
                Err(reason)
            }
        }
    }

    /// The channel of the worker; a connection that is down is tried again first (without the lock held while it
    /// connects, on a thread that may block).
    async fn calls(&self) -> Result<mpsc::UnboundedSender<Call>> {
        let down = match &*self.link.lock().unwrap_or_else(PoisonError::into_inner) {
            Link::Up(calls) if !calls.is_closed() => return Ok(calls.clone()),
            Link::Up(_) => String::from("the people server went away"),
            Link::Down(reason) => reason.clone(),
        };
        let (dial, limits, revisions) =
            (self.dial.clone(), self.limits, Arc::clone(&self.revisions));
        let opened = tokio::task::spawn_blocking(move || open(&dial, limits, &revisions))
            .await
            .map_err(|_| Error::Unreachable(down.clone()))?;
        let mut link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        match opened {
            Ok(calls) => {
                *link = Link::Up(calls.clone());
                Ok(calls)
            }
            Err(reason) => {
                *link = Link::Down(reason.clone());
                Err(Error::Unreachable(reason))
            }
        }
    }

    async fn ask<T>(&self, make: impl FnOnce(Reply<T>) -> Call) -> Result<T> {
        let calls = self.calls().await?;
        let (reply, answer) = oneshot::channel();
        calls.send(make(reply)).map_err(|_| gone())?;
        // The call ends itself at its limit; this is the belt for a worker that does not run it at all.
        let done = match tokio::time::timeout(self.limits.call + GRACE, answer).await {
            Ok(done) => done.map_err(|_| gone())?,
            Err(_) => Err(timed_out(&Fault::TimedOut {
                server: SERVER,
                limit: self.limits.call,
            })),
        };
        // The connection ended under the call: the next call connects again (a server another window started).
        if let Err(Error::Unreachable(text)) = &done
            && text == LOST
        {
            *self.link.lock().unwrap_or_else(PoisonError::into_inner) = Link::Down(LOST.into());
        }
        done
    }
}

/// Start a worker thread that connects and then answers the calls; wait until it is connected.
fn open(
    dial: &Dial,
    limits: Limits,
    revisions: &Revisions,
) -> std::result::Result<mpsc::UnboundedSender<Call>, String> {
    let (calls, inbox) = mpsc::unbounded_channel();
    let (ready, started) = std::sync::mpsc::channel();
    let (dial, shared) = (dial.clone(), Arc::clone(revisions));
    thread::Builder::new()
        .name("people-rpc".into())
        .spawn(move || worker(&dial, limits, inbox, &ready, &shared))
        .map_err(|e| e.to_string())?;
    started.recv().map_err(|e| e.to_string())??;
    Ok(calls)
}

fn gone() -> Error {
    Error::Unreachable("the people server went away".into())
}

#[async_trait]
impl People for RemotePeople {
    fn capabilities(&self) -> Capabilities {
        // The server is the judge: a refusal comes back as the problem it is.
        let mut report = Capabilities::new("fs-people (socket)");
        for id in [
            "person.read",
            "person.create",
            "person.update",
            "person.delete",
        ] {
            report.supported(id);
        }
        report
    }

    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        self.ask(|r| Call::Get(id, r)).await
    }

    async fn list(&self, filter: &ListFilter) -> Result<Vec<StoredObject>> {
        self.ask(|r| Call::List(filter.clone(), r)).await
    }

    async fn create(&self, card: Card) -> Result<StoredObject> {
        let text = serde_json::to_string(&card)?;
        self.ask(|r| Call::Create(text, r)).await
    }

    async fn update(&self, id: ObjectId, card: Card) -> Result<StoredObject> {
        let text = serde_json::to_string(&card)?;
        self.ask(|r| Call::Update(id, text, r)).await
    }

    /// Compare and write in one step at the server, so another window's change is a conflict. The
    /// revision sent is the server's own of the last read of this person (`expected`, computed from a
    /// rebuilt object, would not equal it).
    async fn update_checked(
        &self,
        id: ObjectId,
        _expected: Revision,
        card: Card,
    ) -> Result<StoredObject> {
        let text = serde_json::to_string(&card)?;
        let known = self
            .revisions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
            .copied();
        // Never read here: there is nothing to compare with, so nothing is written blindly.
        let Some(revision) = known else {
            return Err(Error::Changed(id));
        };
        self.ask(|r| Call::UpdateIf(id, revision, text, r)).await
    }

    async fn delete(&self, id: ObjectId) -> Result<()> {
        self.ask(|r| Call::Delete(id, r)).await
    }

    /// The server's list with a term searches name, nickname, e-mail, phone and organisation.
    async fn search(&self, term: &str) -> Result<Vec<StoredObject>> {
        let filter = ListFilter {
            term: Some(term.to_string()),
            ..ListFilter::default()
        };
        self.ask(|r| Call::List(filter, r)).await
    }
}

#[async_trait]
impl RoleBook for RemotePeople {
    async fn roles_of(&self, person: ObjectId) -> Result<Vec<RoleAssignment>> {
        self.ask(|r| Call::RolesOf(person, r)).await
    }

    async fn role(&self, id: &str) -> Result<Option<RoleDefinition>> {
        let id = id.to_string();
        self.ask(|r| Call::Role(id, r)).await
    }
}
