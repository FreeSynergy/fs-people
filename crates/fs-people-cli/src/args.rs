//! The words of the command line.
//!
//! Pattern: Command (pure parse) — no I/O, so every spelling is tested without a server.

use std::path::PathBuf;

/// What to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// All people, one line each; `term` narrows by name, e-mail, phone or organisation.
    List(Option<String>),
    /// One card as JSContact.
    Show(String),
    /// A new person.
    Add {
        /// Full name.
        name: String,
        /// E-mail address.
        email: Option<String>,
    },
    /// Delete a card.
    Delete(String),
    /// People with this e-mail address.
    Find(String),
    /// One card as vCard.
    Export(String),
    /// Import a `.vcf` file.
    Import(PathBuf),
}

/// The help text.
pub const HELP: &str = "fs-people [--socket PATH] <command>

Commands:
  list [--term TEXT]         all people, or those matching TEXT
  show <id>                  one card as JSContact
  add <name> [--email ADDR]  a new person
  delete <id>                delete a card
  find <email>               people with this address
  export <id>                one card as vCard
  import <file.vcf>          import vCards

The socket defaults to $XDG_RUNTIME_DIR/freesynergy/people.sock.";

/// The words, parsed.
#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    /// `--socket`.
    pub socket: Option<PathBuf>,
    /// The command; `None` for `--help`.
    pub command: Option<Command>,
}

/// Parse the words after the program name: options (`--socket PATH`, `--help`) first, then the command.
///
/// # Errors
/// An unknown word, a missing value, a missing id. The message says what is wrong.
pub fn parse(words: &[String]) -> Result<Args, String> {
    // Options come before the command word. After it every word is the command's own, so a title or
    // a value that looks like an option (`-h`, `--socket`) stays what the user typed.
    let mut socket = None;
    let mut at = 0;
    while let Some(word) = words.get(at) {
        match word.as_str() {
            "--help" | "-h" => {
                return Ok(Args {
                    socket,
                    command: None,
                });
            }
            "--socket" => {
                socket = Some(words.get(at + 1).ok_or("--socket needs a path")?.into());
                at += 2;
            }
            _ => break,
        }
    }
    let rest: Vec<&str> = words[at..].iter().map(String::as_str).collect();
    let (name, tail) = rest.split_first().ok_or("no command; try --help")?;
    let one = |what: &str| -> Result<String, String> {
        match tail {
            [x] => Ok((*x).into()),
            _ => Err(format!("{name} needs exactly one {what}")),
        }
    };
    let command = match *name {
        "list" => list(tail)?,
        "show" => Command::Show(one("id")?),
        "delete" => Command::Delete(one("id")?),
        "find" => Command::Find(one("address")?),
        "export" => Command::Export(one("id")?),
        "import" => Command::Import(one("file")?.into()),
        "add" => add(tail)?,
        other => return Err(format!("unknown command {other}; try --help")),
    };
    Ok(Args {
        socket,
        command: Some(command),
    })
}

fn list(tail: &[&str]) -> Result<Command, String> {
    match tail {
        [] => Ok(Command::List(None)),
        ["--term", t] => Ok(Command::List(Some((*t).into()))),
        _ => Err("list takes only --term TEXT".into()),
    }
}

fn add(tail: &[&str]) -> Result<Command, String> {
    match tail {
        [name] if !name.starts_with("--") => Ok(Command::Add {
            name: (*name).into(),
            email: None,
        }),
        [name, "--email", email] if !name.starts_with("--") => Ok(Command::Add {
            name: (*name).into(),
            email: Some((*email).into()),
        }),
        _ => Err("add needs a name, and may take --email ADDR".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn the_socket_option_comes_before_the_command() {
        let a = parse(&words("--socket /x/y list")).unwrap();
        assert_eq!(a.socket, Some("/x/y".into()));
        assert_eq!(a.command, Some(Command::List(None)));
        assert_eq!(
            parse(&words("list --term ada")).unwrap().command,
            Some(Command::List(Some("ada".into())))
        );
        assert_eq!(
            parse(&words("add Ada --email a@x.org")).unwrap().command,
            Some(Command::Add {
                name: "Ada".into(),
                email: Some("a@x.org".into())
            })
        );
        assert_eq!(parse(&words("-h")).unwrap().command, None);
    }

    #[test]
    fn after_the_command_every_word_is_the_commands_own() {
        // `-h` and `--socket` are values here, not options.
        let c = parse(&words("add -h")).unwrap().command;
        assert!(
            matches!(c, Some(Command::Add { ref name, .. }) if name == "-h"),
            "{c:?}"
        );
        assert_eq!(
            parse(&words("show --socket")).unwrap().command,
            Some(Command::Show("--socket".into()))
        );
    }

    #[test]
    fn mistakes_are_named() {
        for bad in [
            "",
            "frob",
            "show",
            "show a b",
            "add",
            "add --email x",
            "add A --email",
            "list x",
            "--socket",
        ] {
            assert!(parse(&words(bad)).is_err(), "{bad:?}");
        }
    }
}
