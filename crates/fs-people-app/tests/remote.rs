//! The window's adapter over the socket: `RemotePeople` against a real server and a real store.
//!
//! Pattern: Integration test — the server runs on its own thread and socket, the adapter is the one
//! the window uses.

use std::path::Path;
use std::sync::Arc;

use fs_components_workbenches::ErrorKind;
use fs_components_workbenches::people::PeopleSource;
use fs_interfaces::people_capnp::people;
use fs_interfaces::rpc::{bind_socket, serve};
use fs_objects::{Store, TursoStore};
use fs_people_app::{NoRoles, PeopleAdapter, RemotePeople};
use fs_people_core::PeopleRepo;
use fs_people_server::PeopleServer;

/// A server on `socket`, on a thread of its own; it ends with the process.
fn start_server(socket: &Path) -> Arc<PeopleRepo> {
    let socket = socket.to_path_buf();
    let (up, started) = std::sync::mpsc::channel::<Arc<PeopleRepo>>();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        tokio::task::LocalSet::new().block_on(&runtime, async move {
            let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
            let repo = Arc::new(PeopleRepo::new(store, "https://local.invalid/users/owner"));
            let root: people::Client =
                PeopleServer::new(repo.clone(), repo.clone(), repo.clone()).into_client();
            let bound = bind_socket(&socket).await.unwrap();
            up.send(repo.clone()).unwrap();
            serve(bound, root.client, std::future::pending()).await.ok();
        });
    });
    started.recv().unwrap()
}

mod common;

use common::person;
use fs_people_core::People;

#[test]
fn create_list_search_read_edit_delete_over_the_socket() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("people.sock");
    start_server(&socket);
    let remote = Arc::new(RemotePeople::connect(&socket).unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let adapter = PeopleAdapter::new(remote.clone(), Arc::new(NoRoles), runtime.handle().clone());

    let ada = runtime
        .block_on(remote.create(person("Ada Lovelace", "ada@example.org")))
        .unwrap();
    runtime
        .block_on(remote.create(person("Grace Hopper", "grace@example.org")))
        .unwrap();
    let key = ada.envelope.id.to_string();

    assert_eq!(adapter.rows("", 0, 100).unwrap().len(), 2);
    let found = adapter.rows("lovelace", 0, 100).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].key, key);
    assert_eq!(adapter.rows("", 1, 100).unwrap().len(), 1, "offset");
    assert_eq!(adapter.rows("", 0, 1).unwrap().len(), 1, "limit");

    let sheet = adapter.read(&key).unwrap();
    assert_eq!(sheet.title, "Ada Lovelace");

    let mut edit = adapter.draft(&key).unwrap();
    let email = edit
        .groups
        .iter_mut()
        .flat_map(|g| g.fields.iter_mut())
        .find(|f| f.value == "ada@example.org")
        .unwrap();
    email.value = "ada@new.example".into();
    adapter.save(&key, &edit).unwrap();
    let shown: String = adapter
        .read(&key)
        .unwrap()
        .sections
        .iter()
        .flat_map(|s| s.fields.iter().map(|f| f.value.clone()))
        .collect();
    assert!(shown.contains("ada@new.example"), "{shown}");

    adapter.delete(&key).unwrap();
    assert_eq!(adapter.rows("", 0, 100).unwrap().len(), 1);
    assert_eq!(adapter.read(&key).unwrap_err().kind, ErrorKind::NotFound);
}

#[test]
fn nothing_on_the_socket_is_said_plainly() {
    let dir = tempfile::tempdir().unwrap();
    let message = RemotePeople::connect(&dir.path().join("none.sock"))
        .err()
        .unwrap();
    assert!(message.contains("fs-people serve"), "{message}");
}

#[test]
fn two_windows_on_one_server_a_stale_save_is_a_conflict_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("people.sock");
    start_server(&socket);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let a_remote = Arc::new(RemotePeople::connect(&socket).unwrap());
    let b_remote = Arc::new(RemotePeople::connect(&socket).unwrap());
    let a = PeopleAdapter::new(
        a_remote.clone(),
        Arc::new(NoRoles),
        runtime.handle().clone(),
    );
    let b = PeopleAdapter::new(b_remote, Arc::new(NoRoles), runtime.handle().clone());
    let ada = runtime
        .block_on(a_remote.create(person("Ada Lovelace", "ada@example.org")))
        .unwrap();
    let key = ada.envelope.id.to_string();

    let mut in_a = a.draft(&key).unwrap();
    let mut in_b = b.draft(&key).unwrap();
    let email = |d: &mut fs_components_workbenches::people::PersonDraft, to: &str| {
        let field = d
            .groups
            .iter_mut()
            .flat_map(|g| g.fields.iter_mut())
            .find(|f| f.value == "ada@example.org")
            .unwrap();
        field.value = to.into();
    };
    email(&mut in_b, "b@example.org");
    b.save(&key, &in_b).unwrap();
    email(&mut in_a, "a@example.org");
    let refused = a.save(&key, &in_a).unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict, "{refused:?}");
    let shown: String = b
        .read(&key)
        .unwrap()
        .sections
        .iter()
        .flat_map(|s| s.fields.iter().map(|f| f.value.clone()))
        .collect();
    assert!(
        shown.contains("b@example.org") && !shown.contains("a@example.org"),
        "{shown}"
    );
}

#[test]
fn the_roles_of_a_person_are_read_over_the_socket_with_their_names() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("people.sock");
    let repo = start_server(&socket);
    let remote = Arc::new(RemotePeople::connect(&socket).unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let adapter = PeopleAdapter::new(remote.clone(), remote.clone(), runtime.handle().clone());
    let ada = runtime
        .block_on(remote.create(person("Ada Lovelace", "ada@example.org")))
        .unwrap();
    runtime
        .block_on(repo.assign_role(
            ada.envelope.id,
            "user",
            Some("https://example.org/groups/team"),
            serde_json::Map::new(),
        ))
        .unwrap();
    let sheet = adapter.read(&ada.envelope.id.to_string()).unwrap();
    let roles: Vec<String> = sheet
        .sections
        .iter()
        .filter(|s| s.title == "Roles")
        .flat_map(|s| s.fields.iter().map(|f| f.value.clone()))
        .collect();
    assert_eq!(roles, ["User"], "{sheet:?}");
}
