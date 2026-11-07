//! The real program says on stderr which stored role files it skipped at the start.

use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

use fs_people_core::CONTEXT;
use fs_objects::{Envelope, Store, StoredObject, TursoStore};
use program::{terminate, wait_for};
use serde_json::json;

#[path = "common/program.rs"]
mod program;

const ME: &str = "https://local.invalid/users/owner";

/// Put role files in a data directory the way the program would have: some readable, some not.
async fn seed(data: &Path, contents: &[serde_json::Value]) {
    std::fs::create_dir(data).unwrap();
    std::fs::set_permissions(data, PermissionsExt::from_mode(0o700)).unwrap();
    let store = TursoStore::open(data.join("objects.db")).await.unwrap();
    for content in contents {
        let mut env = Envelope::new("RoleConfig", "stored");
        env.context = Some(CONTEXT.into());
        let object = StoredObject::new(env, "application/json", content.clone());
        store.put(object, ME).await.unwrap();
    }
}

fn run_and_read_stderr(socket: &Path, data: &Path) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fs-people-server"))
        .arg("--socket")
        .arg(socket)
        .arg("--data")
        .arg(data)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(wait_for(socket));
    let server = program::Server(child);
    // `terminate` waits for exit 0, which closes the pipe, so reading to the end then ends.
    terminate(server);
    let mut text = String::new();
    stderr.read_to_string(&mut text).unwrap();
    text
}

#[tokio::test(flavor = "current_thread")]
async fn each_skipped_role_file_is_one_line_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, data) = (dir.path().join("people.sock"), dir.path().join("data"));
    seed(
        &data,
        &[json!({"nonsense": true}), json!({"also": "nonsense"})],
    )
    .await;
    let text = tokio::task::spawn_blocking(move || run_and_read_stderr(&socket, &data))
        .await
        .unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text:?}");
    for line in lines {
        assert!(
            line.starts_with("fs-people-server: skipped stored role file: role configuration "),
            "{line}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_clean_start_says_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, data) = (dir.path().join("people.sock"), dir.path().join("data"));
    seed(&data, &[]).await;
    let text = tokio::task::spawn_blocking(move || run_and_read_stderr(&socket, &data))
        .await
        .unwrap();
    assert_eq!(text, "");
}
