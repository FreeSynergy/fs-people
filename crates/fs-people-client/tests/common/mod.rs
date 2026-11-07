//! Shared set-up of the tests: the real server on a real Unix socket over the real store, on a
//! thread of its own, and the client connected to it. Nothing here stands in for either.
//!
//! Pattern: Facade over the set-up.
#![allow(dead_code)] // each test file uses a different subset

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_interfaces::rpc::{bind_socket, serve};
use fs_people_client::RpcPeople;
use fs_people_core::{Card, People, PeopleRepo};
use fs_people_server::PeopleServer;
use fs_objects::{Store, TursoStore};
use tokio::sync::oneshot;

pub const ME: &str = "https://local.invalid/users/owner";

/// A server running until this is dropped.
pub struct Served {
    pub socket: PathBuf,
    /// The provider behind the server, for what the client does not offer (set-up only).
    pub repo: Arc<PeopleRepo>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _dir: tempfile::TempDir,
}

impl Served {
    /// Over a fresh in-memory store.
    pub async fn start() -> Self {
        let store = TursoStore::open_in_memory().await.unwrap();
        Self::over(Arc::new(store)).await
    }

    /// Over a plain store file, which refuses sensitive cards.
    pub async fn start_with_file(file: &Path) -> Self {
        let store = TursoStore::open(file).await.unwrap();
        Self::over(Arc::new(store)).await
    }

    async fn over(store: Arc<dyn Store>) -> Self {
        let repo = Arc::new(PeopleRepo::new(store, ME));
        Self::serving(repo.clone(), repo).await
    }

    /// A server whose `People` is this provider and whose roles and feed come from `repo`: for a
    /// provider that does what the real one cannot show, such as never answering.
    pub async fn serving(people: Arc<dyn People>, repo: Arc<PeopleRepo>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("people.sock");
        let (stop, stopped) = oneshot::channel::<()>();
        let (ready, listening) = std::sync::mpsc::channel();
        let (path, repo_here) = (socket.clone(), repo.clone());
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();
            local.block_on(&runtime, async move {
                let bound = bind_socket(&path).await.unwrap();
                let root = PeopleServer::new(people, repo_here.clone(), repo_here);
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
        Self {
            socket,
            repo,
            stop: Some(stop),
            thread: Some(thread),
            _dir: dir,
        }
    }

    /// A client of this server.
    pub fn client(&self) -> RpcPeople {
        RpcPeople::connect(&self.socket).unwrap()
    }

    /// Stop the server; clients stay and find it gone.
    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A person card with one e-mail address.
pub fn person(name: &str, email: &str) -> Card {
    let mut card = Card::individual(name);
    card.add_email(email);
    card
}

/// Create through the client; answers the object id.
pub async fn add(people: &dyn People, card: Card) -> fs_objects::ObjectId {
    people.create(card).await.unwrap().envelope.id
}
