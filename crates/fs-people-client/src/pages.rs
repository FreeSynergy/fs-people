//! `PersonPages`: the people of a filter, one page at a time, continued by a cursor.
//!
//! Pattern: Iterator, external and resumable — a page is one step, the [`Cursor`] is the state that
//! says where the next one starts. The steps sit behind a trait (Strategy), so a frontend depends on
//! `dyn PersonPages` and not on [`RpcPeople`](crate::RpcPeople); [`PageWalk`] (`walk.rs`) is the
//! iterator over it. `People` stays the Repository: this trait is a second, narrower way to read the
//! same list. It lives in the client for now and moves up to `fs-people-core` with a second user
//! (`DESIGN.md`).
//!
//! The total is not known: a page says only whether another follows.

use async_trait::async_trait;
use fs_people_core::{ListFilter, Result};
use fs_objects::StoredObject;

/// Where the next page starts: opaque, handed back as it came. It belongs to the filter it was
/// given with, but the server checks only its form: a cursor of the wrong form is refused, a made-up
/// one of the right form or one from another filter is taken as a place and gives the wrong page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor(Vec<u8>);

impl Cursor {
    /// A cursor from bytes kept elsewhere (or made up, to see the refusal).
    #[must_use]
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// The bytes, as the wire carries them.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// One page of people in the order of the contract.
#[derive(Debug, Clone)]
pub struct PersonPage {
    /// The people of this page.
    pub items: Vec<StoredObject>,
    /// Where the next page starts; `None` when this was the last.
    pub next: Option<Cursor>,
}

/// Page-wise reading of the people list. `limit` and `offset` of the filter are not used: the
/// server decides the size of a page, and the cursor replaces the offset.
#[async_trait]
pub trait PersonPages: Send + Sync {
    /// The first page of what `filter` selects (`kind`, `role`, `term`).
    ///
    /// # Errors
    /// The provider could not be reached or refused the filter.
    async fn first(&self, filter: &ListFilter) -> Result<PersonPage>;

    /// The page that starts at `cursor`, for the same `filter` the cursor came from.
    ///
    /// # Errors
    /// The cursor has the wrong form (`Error::Store(Invalid)`); one of the right form is not
    /// checked against the filter. Otherwise as for `first`.
    async fn next(&self, filter: &ListFilter, cursor: &Cursor) -> Result<PersonPage>;
}
