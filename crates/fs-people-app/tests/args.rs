//! The command line.
//!
//! Pattern: Table of inputs and answers.

use std::path::PathBuf;

use fs_people_app::args::parse;

fn args(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
}

#[test]
fn all_options_are_read_and_nothing_is_the_default() {
    let got = parse(&args(
        "--store a.db --import b.vcf --export c.vcf --snapshot d.png --size 800x600",
    ))
    .unwrap();
    assert_eq!(got.store, Some(PathBuf::from("a.db")));
    assert_eq!(got.import, Some(PathBuf::from("b.vcf")));
    assert_eq!(got.export, Some(PathBuf::from("c.vcf")));
    assert_eq!(got.snapshot, Some(PathBuf::from("d.png")));
    assert_eq!(got.size, (800, 600));
    let none = parse(&[]).unwrap();
    assert_eq!(none.store, None, "in memory");
    assert_eq!(none.size, (1200, 800));
}

#[test]
fn mistakes_are_said() {
    assert!(parse(&args("--nope")).unwrap_err().contains("unknown"));
    assert!(
        parse(&args("--store"))
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
