//! `fs-people`: one binary for everything (ADR 0063). Pattern: Facade over the crates that stay
//! libraries.
//!
//! ```text
//! fs-people                          the window (feature `gui`, on by default)
//! fs-people [--store F] [--size WxH] [--snapshot F.png] [--import F.vcf] [--export F.vcf]
//! fs-people list | show | add | delete | find | export | import     over the socket
//! fs-people serve                    the server (Cap'n Proto, people.capnp)
//! ```

use std::process::ExitCode;

use fs_people_server::ServeError;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("serve") => serve(&args[1..]),
        None | Some("gui") => window(args.get(1..).unwrap_or_default()),
        // `fs-people --socket PATH serve`: the option may stand before the word.
        Some("--socket") if args.get(2).map(String::as_str) == Some("serve") => {
            let mut words = vec!["--socket".to_string(), args[1].clone()];
            words.extend(args[3..].iter().cloned());
            serve(&words)
        }
        Some(_) if has_command(&args) => exit(fs_people_cli::command_line(&args)),
        Some(_) => window(&args),
    }
}

/// Whether the words hold a command (`list`, `show`, ...) rather than only window options.
/// `--socket PATH` belongs to both; every other option with a value is the window's.
fn has_command(words: &[String]) -> bool {
    let mut it = words.iter();
    while let Some(word) = it.next() {
        if word == "--help" || word == "-h" {
            return false;
        }
        if word.starts_with("--") {
            if word != "--local" {
                it.next();
            }
        } else {
            return true;
        }
    }
    false
}

fn exit(code: i32) -> ExitCode {
    u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from)
}

fn serve(words: &[String]) -> ExitCode {
    match fs_people_server::serve(words.iter().map(Into::into)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(ServeError::Usage(m)) => {
            eprintln!("fs-people: {m}");
            ExitCode::from(2)
        }
        Err(ServeError::Failed(m)) => {
            eprintln!("fs-people: {m}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(feature = "gui")]
fn window(words: &[String]) -> ExitCode {
    match fs_people_app::gui::run(words) {
        Ok(()) => ExitCode::SUCCESS,
        Err(m) => {
            eprintln!("fs-people: {m}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(feature = "gui"))]
fn window(_words: &[String]) -> ExitCode {
    eprintln!(
        "fs-people: this build has no window (built with --no-default-features); use a command, e.g. `fs-people list`"
    );
    ExitCode::FAILURE
}
