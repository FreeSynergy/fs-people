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

/// What is wrong with a typed contact value, as a sentence (one per code, ADR 0057).
pub(crate) fn problem(problem: fs_people_core::values::ValueProblem) -> &'static str {
    use fs_people_core::values::ValueProblem as P;
    match problem {
        P::Empty => "Nothing typed.",
        P::MissingAt => "An e-mail address needs an @, like name@example.org.",
        P::ManyAt => "An e-mail address has only one @.",
        P::EmptyLocal => "Type the part before the @.",
        P::EmptyDomain => "Type the part after the @, like example.org.",
        P::LocalCharacter => {
            "The part before the @ holds a character that is not allowed (a space, or a dot at the start, at the end or twice)."
        }
        P::LocalTooLong => "The part before the @ is longer than 64 characters.",
        P::BadDomain => "The part after the @ is not a valid domain name.",
        P::DomainWithoutDot => "The domain needs a dot, like example.org.",
        P::NoCountryCode => "Start with + and the country code (not 0), like +49 30 1234567.",
        P::PhoneCharacter => {
            "A phone number holds digits, and spaces, dots, hyphens or brackets between them."
        }
        P::AddressTooLong => "The whole address is longer than 254 characters.",
        P::PhoneLength => "A phone number has 7 to 15 digits after the +, the first not 0.",
    }
}
