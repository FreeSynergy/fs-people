//! Shared helpers of the integration tests: a real server on a real Unix socket, over the real store.

#![allow(dead_code)] // each test file uses a different subset

use std::sync::Arc;

use fs_interfaces::people_capnp::{people, person, persons, roles};
use fs_interfaces::rpc::{Problem, bind_socket, connect, read_envelope, read_uuid, serve};
use fs_people_core::PeopleRepo;
use fs_people_server::PeopleServer;
use fs_objects::{Store, TursoStore};
use uuid::Uuid;

pub const ME: &str = "https://local.invalid/users/owner";
pub const CLUB: &str = "https://example.org/groups/club";

/// A JSContact card as text.
pub fn card(uid: &str, name: &str, email: &str) -> String {
    format!(
        r#"{{"@type":"Card","version":"1.0","uid":"{uid}","name":{{"full":"{name}"}},"emails":{{"e1":{{"address":"{email}"}}}}}}"#
    )
}

/// Serve this root on a socket of its own while `test` runs against the connection.
pub async fn with_root<F, Fut>(root: people::Client, test: F)
where
    F: FnOnce(people::Client) -> Fut,
    Fut: Future<Output = ()>,
{
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("people.sock");
    let bound = bind_socket(&path).await.unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let serving = tokio::task::spawn_local(serve(bound, root.client, async {
        let _ = stopped.await;
    }));
    let connection = connect::<people::Client>(&path).await.unwrap();
    test(connection.client.clone()).await;
    drop(connection);
    let _ = stop.send(());
    serving.await.unwrap().unwrap();
}

/// The store-backed provider over a fresh in-memory store, behind a real socket.
pub async fn with_server<F, Fut>(test: F)
where
    F: FnOnce(people::Client) -> Fut,
    Fut: Future<Output = ()>,
{
    with_store(|root, _| test(root)).await;
}

/// Like [`with_server`], and the test also reaches the store itself.
pub async fn with_store<F, Fut>(test: F)
where
    F: FnOnce(people::Client, Arc<dyn Store>) -> Fut,
    Fut: Future<Output = ()>,
{
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
            let repo = Arc::new(PeopleRepo::new(store.clone(), ME));
            let root = PeopleServer::new(repo.clone(), repo.clone(), repo).into_client();
            with_root(root, |root| test(root, store)).await;
        })
        .await;
}

pub async fn persons_of(root: &people::Client) -> persons::Client {
    let answer = root.persons_request().send().promise.await.unwrap();
    answer.get().unwrap().get_persons().unwrap()
}

pub async fn roles_of(root: &people::Client) -> roles::Client {
    let answer = root.roles_request().send().promise.await.unwrap();
    answer.get().unwrap().get_roles().unwrap()
}

/// The id of a person on the wire, and that its answer holds no role.
pub fn id_of(person: person::Reader<'_>) -> Uuid {
    assert_eq!(person.get_roles().unwrap().len(), 0, "a role in a Person");
    read_envelope(person.get_envelope().unwrap())
        .unwrap()
        .id
        .as_uuid()
}

pub fn name_of(person: person::Reader<'_>) -> String {
    read_envelope(person.get_envelope().unwrap()).unwrap().name
}

pub fn json_of(person: person::Reader<'_>) -> serde_json::Value {
    serde_json::from_str(person.get_jscontact().unwrap().to_str().unwrap()).unwrap()
}

/// Create a card and answer its object id.
pub async fn create(persons: &persons::Client, uid: &str, name: &str, email: &str) -> Uuid {
    let mut req = persons.create_request();
    req.get().set_jscontact(card(uid, name, email).as_str());
    let answer = req.send().promise.await.unwrap();
    id_of(answer.get().unwrap().get_person().unwrap())
}

/// A list of uuids from the wire.
pub fn uuids(
    list: capnp::struct_list::Reader<'_, fs_interfaces::common_capnp::uuid::Owned>,
) -> Vec<Uuid> {
    list.iter().map(|u| read_uuid(u).unwrap()).collect()
}

/// Status and exact type URI of the problems a consumer can tell apart.
pub type Kind = (u16, &'static str);
pub const NOT_FOUND: Kind = (404, "https://freesynergy.net/problems/people/not-found");
pub const CONFLICT: Kind = (409, "https://freesynergy.net/problems/people/conflict");
pub const INVALID: Kind = (422, "https://freesynergy.net/problems/people/invalid");
pub const SENSITIVE: Kind = (403, "https://freesynergy.net/problems/people/sensitive");
pub const STORED_DATA: Kind = (500, "https://freesynergy.net/problems/people/stored-data");
pub const BAD_REQUEST: Kind = (422, "https://freesynergy.net/problems/invalid-request");
pub const TOO_LARGE: Kind = (413, "https://freesynergy.net/problems/too-large");
pub const NOT_POSSIBLE: Kind = (501, "https://freesynergy.net/problems/unsupported");

/// A call that had to fail with exactly this problem; answers its detail.
#[track_caller]
pub fn expect_problem<T>(answer: Result<T, capnp::Error>, kind: Kind) -> String {
    let problem = problem_of(answer);
    assert_eq!(
        (problem.status, problem.problem_type.as_str()),
        kind,
        "{}",
        problem.detail
    );
    problem.detail
}

/// The problem of a call that had to fail.
#[track_caller]
pub fn problem_of<T>(answer: Result<T, capnp::Error>) -> Problem {
    let Err(error) = answer else {
        panic!("the call succeeded");
    };
    Problem::from_error(&error).unwrap_or_else(|| panic!("no problem in: {error}"))
}

/// An id nothing has.
pub fn unknown_id() -> Uuid {
    fs_objects::Envelope::new("x", "x").id.as_uuid()
}
