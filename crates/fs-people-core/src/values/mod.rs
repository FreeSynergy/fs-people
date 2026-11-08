//! Contact values that are valid when they exist: [`EmailAddress`], [`PhoneNumber`].
//!
//! Pattern: Value object (coding.md §2) — small, immutable, compared by value, valid on construction.
//! The trait [`ContactValue`] is what a form needs of one: make it from what was typed, or say why
//! not, as a [`ValueProblem`] code. The code is not a sentence: the program's words turn it into
//! one (ADR 0057). A frontend checks as the person types, the core checks again at intake.

mod email;
mod phone;

pub use email::EmailAddress;
pub use phone::PhoneNumber;

/// Why typed text is not (yet) a value. The variants are stable codes for the words of a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueProblem {
    /// Nothing typed.
    Empty,
    /// An e-mail address needs an `@`.
    MissingAt,
    /// More than one `@`.
    ManyAt,
    /// Nothing before the `@`.
    EmptyLocal,
    /// Nothing after the `@`.
    EmptyDomain,
    /// A character that may not stand before the `@`.
    LocalCharacter,
    /// The part before the `@` is longer than 64 characters.
    LocalTooLong,
    /// The domain is not a valid host name (also after conversion from a Unicode name).
    BadDomain,
    /// The domain has no dot: it names no public host.
    DomainWithoutDot,
    /// A phone number needs `+`, the country code and the number (E.164).
    NoCountryCode,
    /// A character a phone number does not hold.
    PhoneCharacter,
    /// Fewer than 7 or more than 15 digits.
    PhoneLength,
}

/// A value of a contact point, valid by construction.
pub trait ContactValue: Sized {
    /// From what was typed (spaces around it do not matter).
    ///
    /// # Errors
    /// The first rule the text breaks.
    fn parse(typed: &str) -> Result<Self, ValueProblem>;

    /// The canonical text to store and compare.
    fn as_str(&self) -> &str;

    /// Whether `typed` is a value, without making one; what a field asks on every key.
    fn check(typed: &str) -> Result<(), ValueProblem> {
        Self::parse(typed).map(|_| ())
    }
}
