//! `RemotePeople`: `People` over the Cap'n Proto socket of the people server (ADR 0063).
//!
//! Pattern: Proxy — the same contract as the in-process core, answered by the server. A Cap'n Proto
//! client lives on one thread, so a worker thread owns the connection and the proxy hands it calls
//! through a channel; the window thread waits for the answer like for the in-process core.
//!
//! Provisional: only what the window needs (read, list, search, create, update, delete). Roles are
//! not asked, so the window shows none over the socket (`RoleBook` is not implemented here).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use async_trait::async_trait;
use fs_interfaces::common_capnp;
use fs_interfaces::people_capnp::{people, person, persons};
use fs_interfaces::rpc::{Problem, connect, read_envelope, write_uuid};
use fs_interfaces::{Capabilities, Unsupported};
use fs_objects::{ObjectId, StoredObject};
use fs_people_core::{Card, Error, ListFilter, People, Result, Revision};
use tokio::sync::{mpsc, oneshot};

type Reply<T> = oneshot::Sender<Result<T>>;

/// The server's revision of each person as last read here. The adapter reads a person right before it
/// writes, so the entry is the revision of what it compared; a rebuilt object could not give it back.
type Revisions = Arc<Mutex<HashMap<ObjectId, u64>>>;

enum Call {
    Get(ObjectId, Reply<Option<StoredObject>>),
    List(ListFilter, Reply<Vec<StoredObject>>),
    Create(String, Reply<StoredObject>),
    Update(ObjectId, String, Reply<StoredObject>),
    UpdateIf(ObjectId, u64, String, Reply<StoredObject>),
    Delete(ObjectId, Reply<()>),
}

/// The people server behind a socket, as a provider of [`People`].
pub struct RemotePeople {
    calls: mpsc::UnboundedSender<Call>,
    revisions: Revisions,
}

impl RemotePeople {
    /// Connect to the server on `socket`.
    ///
    /// # Errors
    /// The text for the person when nothing answers on the socket.
    pub fn connect(socket: &Path) -> std::result::Result<Self, String> {
        let (calls, inbox) = mpsc::unbounded_channel();
        let (ready, started) = std::sync::mpsc::channel();
        let socket = socket.to_path_buf();
        let revisions = Revisions::default();
        let shared = Arc::clone(&revisions);
        thread::Builder::new()
            .name("people-rpc".into())
            .spawn(move || worker(&socket, inbox, &ready, &shared))
            .map_err(|e| e.to_string())?;
        started.recv().map_err(|e| e.to_string())??;
        Ok(Self { calls, revisions })
    }

    async fn ask<T>(&self, make: impl FnOnce(Reply<T>) -> Call) -> Result<T> {
        let (reply, answer) = oneshot::channel();
        self.calls.send(make(reply)).map_err(|_| gone())?;
        answer.await.map_err(|_| gone())?
    }
}

fn gone() -> Error {
    Error::Stored("the people server went away".into())
}

fn worker(
    socket: &Path,
    mut inbox: mpsc::UnboundedReceiver<Call>,
    ready: &std::sync::mpsc::Sender<std::result::Result<(), String>>,
    revisions: &Revisions,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async move {
        let connection = match connect::<people::Client>(socket).await {
            Ok(c) => c,
            Err(e) => {
                let _ = ready.send(Err(format!(
                    "no people server on {}: {e}; start it with `fs-people serve`",
                    socket.display()
                )));
                return;
            }
        };
        let persons = match connection.client.persons_request().send().promise.await {
            Ok(r) => match r
                .get()
                .and_then(people::persons_results::Reader::get_persons)
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = ready.send(Err(e.to_string()));
                    return;
                }
            },
            Err(e) => {
                let _ = ready.send(Err(e.to_string()));
                return;
            }
        };
        let _ = ready.send(Ok(()));
        while let Some(call) = inbox.recv().await {
            let (persons, revisions) = (persons.clone(), Arc::clone(revisions));
            tokio::task::spawn_local(async move { carry_out(&persons, &revisions, call).await });
        }
    });
}

async fn carry_out(persons: &persons::Client, revisions: &Revisions, call: Call) {
    match call {
        Call::Get(id, reply) => {
            let _ = reply.send(get(persons, revisions, id).await);
        }
        Call::List(filter, reply) => {
            let _ = reply.send(list(persons, revisions, &filter).await);
        }
        Call::Create(text, reply) => {
            let _ = reply.send(create(persons, revisions, &text).await);
        }
        Call::Update(id, text, reply) => {
            let _ = reply.send(update(persons, revisions, id, &text).await);
        }
        Call::UpdateIf(id, revision, text, reply) => {
            let _ = reply.send(update_if(persons, revisions, id, revision, &text).await);
        }
        Call::Delete(id, reply) => {
            let _ = reply.send(delete(persons, id).await);
        }
    }
}

/// What the server said, in the core's terms. The server's own words are kept.
fn problem(error: &capnp::Error, id: Option<ObjectId>) -> Error {
    match Problem::from_error(error) {
        Some(p) if p.is_unsupported() => Error::Unsupported(Unsupported::new("people", p.detail)),
        Some(p) if p.status == 404 => id.map_or(Error::Conflict(p.detail), Error::NotFound),
        // The server's compare-and-set says `conflict:` and names no revision.
        Some(p) if p.detail.starts_with("conflict:") && id.is_some() => {
            id.map_or(Error::Conflict(p.detail), Error::Changed)
        }
        Some(p) => Error::Conflict(p.detail),
        None => Error::Stored(error.to_string()),
    }
}

fn stored(revisions: &Revisions, p: person::Reader<'_>) -> capnp::Result<StoredObject> {
    let envelope = read_envelope(p.get_envelope()?)?;
    let content = serde_json::from_str(&p.get_jscontact()?.to_string()?)
        .map_err(|e| capnp::Error::failed(e.to_string()))?;
    let id = envelope.id;
    revisions
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(id, p.get_revision());
    Ok(StoredObject::new(envelope, "application/json", content))
}

async fn get(
    persons: &persons::Client,
    revisions: &Revisions,
    id: ObjectId,
) -> Result<Option<StoredObject>> {
    let mut request = persons.get_request();
    write_uuid(request.get().init_id(), Some(id.as_uuid()));
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, Some(id)))?;
    let r = answer.get().map_err(|e| problem(&e, Some(id)))?;
    if !r.get_found() {
        return Ok(None);
    }
    let person = r
        .get_person()
        .and_then(|p| stored(revisions, p))
        .map_err(|e| problem(&e, Some(id)))?;
    Ok(Some(person))
}

async fn list(
    persons: &persons::Client,
    revisions: &Revisions,
    filter: &ListFilter,
) -> Result<Vec<StoredObject>> {
    let (mut found, mut cursor) = (Vec::new(), Vec::<u8>::new());
    let mut skip = filter.offset;
    if filter.limit == Some(0) {
        return Ok(found);
    }
    loop {
        let mut request = persons.list_request();
        {
            let mut wire = request.get().init_filter();
            if let Some(term) = &filter.term {
                wire.set_term(term.as_str());
            }
            if let Some(kind) = &filter.kind {
                wire.set_kind(kind.as_str());
            }
            if let Some(role) = &filter.role {
                wire.set_role(role.as_str());
            }
        }
        request.get().init_cursor().set_opaque(&cursor);
        let answer = request
            .send()
            .promise
            .await
            .map_err(|e| problem(&e, None))?;
        let page = answer
            .get()
            .and_then(persons::list_results::Reader::get_page)
            .map_err(|e| problem(&e, None))?;
        for item in page.get_items().map_err(|e| problem(&e, None))? {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            found.push(stored(revisions, item).map_err(|e| problem(&e, None))?);
            if filter.limit.is_some_and(|limit| found.len() >= limit) {
                return Ok(found);
            }
        }
        let next = page
            .get_next()
            .and_then(common_capnp::cursor::Reader::get_opaque)
            .map_err(|e| problem(&e, None))?;
        if next.is_empty() {
            return Ok(found);
        }
        cursor = next.to_vec();
    }
}

async fn create(
    persons: &persons::Client,
    revisions: &Revisions,
    card: &str,
) -> Result<StoredObject> {
    let mut request = persons.create_request();
    request.get().set_jscontact(card);
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, None))?;
    answer
        .get()
        .and_then(persons::create_results::Reader::get_person)
        .and_then(|p| stored(revisions, p))
        .map_err(|e| problem(&e, None))
}

async fn update(
    persons: &persons::Client,
    revisions: &Revisions,
    id: ObjectId,
    card: &str,
) -> Result<StoredObject> {
    let mut request = persons.update_request();
    write_uuid(request.get().init_id(), Some(id.as_uuid()));
    request.get().set_jscontact(card);
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, Some(id)))?;
    answer
        .get()
        .and_then(persons::update_results::Reader::get_person)
        .and_then(|p| stored(revisions, p))
        .map_err(|e| problem(&e, Some(id)))
}

async fn update_if(
    persons: &persons::Client,
    revisions: &Revisions,
    id: ObjectId,
    revision: u64,
    card: &str,
) -> Result<StoredObject> {
    let mut request = persons.update_if_request();
    write_uuid(request.get().init_id(), Some(id.as_uuid()));
    request.get().set_revision(revision);
    request.get().set_jscontact(card);
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| problem(&e, Some(id)))?;
    answer
        .get()
        .and_then(persons::update_if_results::Reader::get_person)
        .and_then(|p| stored(revisions, p))
        .map_err(|e| problem(&e, Some(id)))
}

async fn delete(persons: &persons::Client, id: ObjectId) -> Result<()> {
    let mut request = persons.delete_request();
    write_uuid(request.get().init_id(), Some(id.as_uuid()));
    request
        .send()
        .promise
        .await
        .map(|_| ())
        .map_err(|e| problem(&e, Some(id)))
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

/// A role book that has nothing to say: every method answers "not possible" (ADR 0045), so the
/// window shows no roles. Used with [`RemotePeople`] until the proxy asks the server for roles.
pub struct NoRoles;

#[async_trait]
impl fs_people_core::RoleBook for NoRoles {}
