//! The real program: started, used, stopped and started again; what it holds is still there.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use fs_interfaces::people_capnp::people;
use fs_interfaces::rpc::{Problem, connect, read_uuid, write_uuid};
use serde_json::json;

const SCOPE: &str = "https://example.org/groups/club";

fn start(socket: &Path, data: &Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_fs-people-server"))
        .arg("--socket")
        .arg(socket)
        .arg("--data")
        .arg(data)
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

async fn wait_for(socket: &Path) {
    for _ in 0..200 {
        if tokio::net::UnixStream::connect(socket).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the server did not come up");
}

fn terminate(mut child: Child) {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(child.id().to_string())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(child.wait().unwrap().success());
}

fn sensitive_card() -> String {
    json!({"@type": "Card", "version": "1.0", "uid": "secret", "name": {"full": "Secret"},
        "freesynergy.net:sensitive": ["pd:HealthRecord"]})
    .to_string()
}

/// Use the first run: a card with a role, a configuration, and a refused sensitive card.
async fn fill(socket: &Path) {
    let connection = connect::<people::Client>(socket).await.unwrap();
    let root = &connection.client;
    let answer = root.persons_request().send().promise.await.unwrap();
    let persons = answer.get().unwrap().get_persons().unwrap();
    let answer = root.roles_request().send().promise.await.unwrap();
    let roles = answer.get().unwrap().get_roles().unwrap();

    let mut create = persons.create_request();
    let kept = json!({"@type": "Card", "version": "1.0", "uid": "kept", "name": {"full": "Kept"}});
    create.get().set_jscontact(kept.to_string().as_str());
    let answer = create.send().promise.await.unwrap();
    let envelope = answer
        .get()
        .unwrap()
        .get_person()
        .unwrap()
        .get_envelope()
        .unwrap();
    let id = read_uuid(envelope.get_id().unwrap());

    let config = json!([
        {"id": "referee", "names": {"en": "Referee"}, "owner": "fs-groups", "scope": "group"}
    ]);
    let mut load = roles.load_config_request();
    load.get().set_json(config.to_string().as_str());
    load.send().promise.await.unwrap();

    let mut assign = roles.assign_request();
    write_uuid(assign.get().init_person(), id);
    assign.get().set_role("referee");
    assign.get().set_scope(SCOPE);
    assign.send().promise.await.unwrap();

    // The store of this program is not encrypted, so ADR 0029 refuses: typed, and nothing is stored.
    let mut secret = persons.create_request();
    secret.get().set_jscontact(sensitive_card().as_str());
    let refused = secret.send().promise.await.err().unwrap();
    let refused = Problem::from_error(&refused).unwrap();
    assert!(
        refused
            .problem_type
            .ends_with("/people/encryption-required")
    );
    assert_eq!(refused.status, 409);
}

/// The second run sees all of it.
async fn check(socket: &Path) {
    let connection = connect::<people::Client>(socket).await.unwrap();
    let root = &connection.client;
    let answer = root.persons_request().send().promise.await.unwrap();
    let persons = answer.get().unwrap().get_persons().unwrap();
    let answer = root.roles_request().send().promise.await.unwrap();
    let roles = answer.get().unwrap().get_roles().unwrap();

    let listed = persons.list_request().send().promise.await.unwrap();
    let items = listed
        .get()
        .unwrap()
        .get_page()
        .unwrap()
        .get_items()
        .unwrap();
    assert_eq!(items.len(), 1);
    let id = read_uuid(items.get(0).get_envelope().unwrap().get_id().unwrap());

    let mut of = roles.roles_of_request();
    write_uuid(of.get().init_person(), id);
    let answer = of.send().promise.await.unwrap();
    let held = answer.get().unwrap().get_items().unwrap();
    assert_eq!(held.get(0).get_role().unwrap().to_str().unwrap(), "referee");

    let mut get = roles.get_request();
    get.get().set_id("referee");
    let answer = get.send().promise.await.unwrap();
    assert!(
        answer.get().unwrap().get_found(),
        "the configuration is gone"
    );
    let mut shipped = roles.get_request();
    shipped.get().set_id("customer");
    let answer = shipped.send().promise.await.unwrap();
    assert!(
        !answer.get().unwrap().get_found(),
        "the configuration was not swapped"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn people_survive_a_restart_and_a_second_start_is_refused() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let socket = dir.path().join("people.sock");
            let data = dir.path().join("data");

            let first = start(&socket, &data);
            wait_for(&socket).await;

            let mut second = start(&socket, &data);
            assert!(!second.wait().unwrap().success(), "a second server started");
            assert!(socket.exists(), "the second start touched the first");

            fill(&socket).await;
            terminate(first);
            assert!(!socket.exists(), "the socket stayed behind");

            let again = start(&socket, &data);
            wait_for(&socket).await;
            check(&socket).await;
            terminate(again);
        })
        .await;
}
