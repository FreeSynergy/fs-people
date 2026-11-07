//! Shared helpers of the tests: the real server on a real Unix socket over the real store (on a
//! thread of its own), the real client, and the real adapter over it. Nothing here pretends to be
//! any of them.
//!
//! Pattern: Facade over the set-up.
#![allow(dead_code)] // each test file uses a different subset

mod counting;

pub use counting::Counting;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_components_workbenches::people::{PeopleLabels, PeopleSource, PeopleState, Variant};
use fs_interfaces::rpc::{bind_socket, serve};
use fs_people_app::PeopleAdapter;
use fs_people_client::RpcPeople;
use fs_people_core::{Card, People, PeopleRepo, card_of};
use fs_people_server::PeopleServer;
use fs_objects::{ObjectId, Store, TursoStore};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

pub const ME: &str = "https://local.invalid/users/owner";

/// A server on its own thread until this is dropped.
struct Served {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _dir: tempfile::TempDir,
}

impl Served {
    fn start(repo: &Arc<PeopleRepo>) -> (Self, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("people.sock");
        let (stop, stopped) = oneshot::channel::<()>();
        let (ready, listening) = std::sync::mpsc::channel();
        let (path, provider) = (socket.clone(), repo.clone());
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            tokio::task::LocalSet::new().block_on(&runtime, async move {
                let bound = bind_socket(&path).await.unwrap();
                let root = PeopleServer::new(provider.clone(), provider.clone(), provider);
                ready.send(()).unwrap();
                let shutdown = async {
                    let _ = stopped.await;
                };
                serve(bound, root.into_client().client, shutdown)
                    .await
                    .unwrap();
            });
        });
        listening.recv().unwrap();
        let served = Self {
            stop: Some(stop),
            thread: Some(thread),
            _dir: dir,
        };
        (served, socket)
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A runtime, the server and its store, the client, and the adapter over the client. Dropped in the
/// order of the fields: the client goes before the server.
pub struct World {
    pub adapter: Arc<PeopleAdapter>,
    /// The client: what the app talks to.
    pub people: Arc<RpcPeople>,
    /// The same client as `PersonPages`, counting its calls (a Decorator; the server is the real one).
    pub pages: Arc<Counting>,
    /// Where the server listens, for the binary.
    pub socket: PathBuf,
    /// The provider behind the server, only for what the client does not offer (marking an AI,
    /// importing JSContact): set-up, never what is under test.
    pub repo: Arc<PeopleRepo>,
    _served: Served,
    pub runtime: Runtime,
}

impl World {
    /// Over a fresh in-memory store.
    pub fn in_memory() -> Self {
        let runtime = Runtime::new().unwrap();
        let store = runtime.block_on(TursoStore::open_in_memory()).unwrap();
        Self::over(runtime, Arc::new(store))
    }

    /// Over a plain store file, which refuses sensitive cards.
    pub fn in_file(path: &Path) -> Self {
        let runtime = Runtime::new().unwrap();
        let store = runtime.block_on(TursoStore::open(path)).unwrap();
        Self::over(runtime, Arc::new(store))
    }

    fn over(runtime: Runtime, store: Arc<dyn Store>) -> Self {
        let repo = Arc::new(PeopleRepo::new(store, ME));
        let (served, socket) = Served::start(&repo);
        let people = Arc::new(RpcPeople::connect(&socket).unwrap());
        let pages = Arc::new(Counting::over(people.clone()));
        let adapter = Arc::new(PeopleAdapter::new(
            people.clone(),
            people.clone(),
            pages.clone(),
            people.clone(),
            runtime.handle().clone(),
        ));
        Self {
            adapter,
            people,
            pages,
            socket,
            repo,
            _served: served,
            runtime,
        }
    }

    /// Store `card` through the client; returns the row key.
    pub fn add(&self, card: Card) -> String {
        let stored = self.runtime.block_on(self.people.create(card)).unwrap();
        stored.envelope.id.to_string()
    }

    /// The card as stored now.
    pub fn card(&self, key: &str) -> Card {
        let id: ObjectId = key.parse().unwrap();
        let stored = self.runtime.block_on(self.people.get(id)).unwrap().unwrap();
        card_of(&stored).unwrap()
    }

    /// The state machine over the real client.
    pub fn state(&self) -> PeopleState {
        let source: Arc<dyn PeopleSource> = self.adapter.clone();
        PeopleState::new(source, fs_people_app::labels(), Variant::Full)
    }

    /// The labels, for a test that wants to change them.
    pub fn labels() -> PeopleLabels {
        fs_people_app::labels()
    }
}

/// A person card with one e-mail address.
pub fn person(name: &str, email: &str) -> Card {
    let mut card = Card::individual(name);
    card.add_email(email);
    card
}

/// A folder for the files of one test, removed when the test ends (also when it fails).
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("fs-people-app-{}", ObjectId::new()));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
