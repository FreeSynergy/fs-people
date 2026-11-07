//! The words of the program.
//!
//! Pattern: none — constants (whole sentences with named placeholders, `{name}`) and one pure
//! function that fills them. All English text of this frontend is in this file; translation
//! (ADR 0053) will replace its content, not its users.

use fs_components_workbenches::people::PeopleLabels;

/// A sentence with its named placeholders (`{name}`) filled from `values`; the sentence stays whole,
/// so a language may put the parts in another order.
pub(crate) fn fill(sentence: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(sentence.to_string(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

/// What the command line says to a person.
pub(crate) mod cli {
    pub const NO_SOCKET: &str = "no --socket given: the app talks to a running people server; start one with `fs-people-server --socket <path>` and pass the same path here";
    pub const STORE_GONE: &str = "--store is gone: the store belongs to the server, start it with `fs-people-server --data <dir>` and give the app its --socket";
    pub const NEEDS_VALUE: &str = "{option} needs a value";
    pub const UNKNOWN_OPTION: &str = "unknown option {option}";
    pub const SENSITIVE_NEEDS_EXPORT: &str = "--export-sensitive needs --export";
    pub const CANNOT_REACH: &str = "cannot reach the people server at {socket}: {error}";
    pub const NOT_EXPORTED: &str = "{count} could not be exported";
    pub const WROTE: &str = "wrote {file}";
}

/// What the adapter says to the workbench.
pub(crate) mod said {
    pub const GONE: &str = "This person no longer exists.";
    pub const NAMES_NOBODY: &str = "\"{key}\" names nobody.";
    pub const CHANGED: &str = "Someone else changed this person since you started.";
    pub const NEEDS_ENCRYPTION: &str = "this card is marked sensitive and this store is not encrypted, so it was refused ({error})";
}

/// The title of the window.
pub const TITLE: &str = "FreeSynergy People";

/// The words of the workbench.
pub fn labels() -> PeopleLabels {
    let mut labels = PeopleLabels::default();
    labels.search = "Search people".into();
    labels.columns = vec!["Name".into(), "E-mail".into()];
    labels
}

/// Headings and labels of what is shown about a person.
pub(crate) mod field {
    pub const AI: &str = "AI";
    pub const SENSITIVE: &str = "Sensitive: {}";
    pub const SENSITIVE_ROW: &str = "Sensitive";
    pub const UNCLEAR: &str = "categories unclear";
    pub const CONTACT: &str = "Contact";
    pub const WORK: &str = "Organisation";
    pub const PLACES: &str = "Addresses";
    pub const ABOUT: &str = "About";
    pub const ROLES: &str = "Roles";
    pub const KEPT: &str = "Kept on the card";
    pub const NAME: &str = "Name";
    pub const EMAIL: &str = "E-mail";
    pub const PHONE: &str = "Phone";
    pub const ORGANISATION: &str = "Organisation";
    pub const JOB: &str = "Title";
    pub const ADDRESS: &str = "Address";
    pub const NICKNAME: &str = "Nickname";
    pub const LINK: &str = "Link";
    pub const NOTE: &str = "Note";
    pub const KIND: &str = "Kind";
    pub const ROLE: &str = "Role";
    pub const OTHER: &str = "Not shown here";
    pub const UNREADABLE: &str = "could not be read: {}";
}

/// `--socket` was not given.
pub fn no_socket() -> String {
    cli::NO_SOCKET.to_string()
}

/// The app could not connect to the server.
pub fn cannot_reach(socket: &str, error: &str) -> String {
    fill(cli::CANNOT_REACH, &[("socket", socket), ("error", error)])
}

/// Some people could not be exported.
pub fn not_exported(count: usize) -> String {
    fill(cli::NOT_EXPORTED, &[("count", &count.to_string())])
}

/// A picture was written.
pub fn wrote(file: &str) -> String {
    fill(cli::WROTE, &[("file", file)])
}
