//! The words of the program.
//!
//! Pattern: Value objects — all English text of this frontend in one place; translation (ADR 0053)
//! will replace this file's content, not its users.

use fs_components_workbenches::people::PeopleLabels;

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
