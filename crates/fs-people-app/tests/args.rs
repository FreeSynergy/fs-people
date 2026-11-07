//! The command line.
//!
//! A table of inputs and the answers expected.

use std::path::PathBuf;

use fs_people_app::args::parse;

fn args(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
}

#[test]
fn all_options_are_read_and_nothing_is_the_default() {
    let got = parse(&args(
        "--socket a.sock --import b.vcf --export c.vcf --snapshot d.png --size 800x600",
    ))
    .unwrap();
    assert_eq!(got.socket, Some(PathBuf::from("a.sock")));
    assert_eq!(got.import, Some(PathBuf::from("b.vcf")));
    assert_eq!(got.export, Some(PathBuf::from("c.vcf")));
    assert_eq!(got.snapshot, Some(PathBuf::from("d.png")));
    assert_eq!(got.size, (800, 600));
    let none = parse(&[]).unwrap();
    assert_eq!(none.socket, None, "no socket, no server");
    assert_eq!(none.size, (1200, 800));
}

#[test]
fn mistakes_are_said() {
    assert!(parse(&args("--nope")).unwrap_err().contains("unknown"));
    assert!(
        parse(&args("--socket"))
            .unwrap_err()
            .contains("needs a value")
    );
    for bad in ["10", "axb", "0x5", "5x0"] {
        assert!(
            parse(&args(&format!("--size {bad}")))
                .unwrap_err()
                .contains("WIDTHxHEIGHT")
        );
    }
}

#[test]
fn export_sensitive_is_repeatable_and_needs_an_export() {
    let got = parse(&args(
        "--export o.vcf --export-sensitive a --export-sensitive b",
    ))
    .unwrap();
    assert_eq!(got.export_sensitive, ["a", "b"]);
    assert!(
        parse(&args("--export-sensitive a"))
            .unwrap_err()
            .contains("needs --export")
    );
}

#[test]
fn the_app_needs_a_socket_and_says_where_it_comes_from() {
    let none = parse(&[]).unwrap();
    let said = none.socket_path().unwrap_err();
    assert!(said.contains("fs-people-server --socket"), "{said}");
    let some = parse(&args("--socket x.sock")).unwrap();
    assert_eq!(some.socket_path().unwrap(), std::path::Path::new("x.sock"));
}

#[test]
fn the_store_belongs_to_the_server_now() {
    let said = parse(&args("--store a.db")).unwrap_err();
    assert!(
        said.contains("fs-people-server") && said.contains("--data"),
        "{said}"
    );
}
