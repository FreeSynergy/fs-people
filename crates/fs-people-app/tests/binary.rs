//! The real binary against the real server: it connects by `--socket`, and without one it says how.
//!
//! Pattern: Integration tests of the process; no window is opened (export only).

mod common;

use std::process::Command;

use common::{Scratch, World, person};
use fs_components_workbenches::people::PeopleSource;

fn app() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fs-people-app"))
}

#[test]
fn without_a_socket_it_stops_and_names_the_server() {
    let out = app().output().unwrap();
    assert!(!out.status.success());
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("fs-people-server --socket"), "{said}");
}

#[test]
fn a_socket_nobody_listens_on_is_an_error_in_words() {
    let dir = Scratch::new();
    let out = app()
        .arg("--socket")
        .arg(dir.file("none.sock"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("cannot reach the people server"), "{said}");
}

#[test]
fn it_imports_and_exports_through_the_server() {
    let w = World::in_memory();
    w.add(person("Ada", "ada@x.org"));
    let dir = Scratch::new();
    let (out_file, vcf) = (dir.file("out.vcf"), dir.file("in.vcf"));
    std::fs::write(
        &vcf,
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Grace\r\nEMAIL:g@x.org\r\nEND:VCARD\r\n",
    )
    .unwrap();
    let out = app()
        .arg("--socket")
        .arg(&w.socket)
        .arg("--import")
        .arg(&vcf)
        .arg("--export")
        .arg(&out_file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = std::fs::read_to_string(&out_file).unwrap();
    assert!(
        text.contains("FN:Ada") && text.contains("FN:Grace"),
        "{text}"
    );
    assert_eq!(
        w.adapter.rows("", 0, 10).unwrap().len(),
        2,
        "the server holds both"
    );
}
