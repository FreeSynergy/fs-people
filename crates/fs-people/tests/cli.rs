//! The words of the real program: `--version`, and `--facet` (known, unknown) for the server and the window.

use std::process::Command;

fn program() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fs-people"))
}

#[test]
fn version_prints_the_name_and_the_version() {
    let out = program().arg("--version").output().unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        concat!("fs-people ", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn the_server_refuses_a_facet_this_program_does_not_have() {
    let dir = tempfile::tempdir().unwrap();
    let out = program()
        .args(["serve", "--facet", "gallery", "--socket"])
        .arg(dir.path().join("people.sock"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains("unknown facet gallery") && said.contains("people"),
        "{said}"
    );
}

#[test]
fn the_window_refuses_a_facet_this_program_does_not_have() {
    let out = program().args(["--facet", "gallery"]).output().unwrap();
    assert!(!out.status.success());
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("unknown facet gallery"), "{said}");
}
