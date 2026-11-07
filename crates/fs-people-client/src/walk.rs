//! `PageWalk`: the pages of a filter, one `next_page` after the other.
//!
//! Pattern: Iterator (async, so not `std::iter::Iterator`) over [`PersonPages`], with State: the
//! walk is at the start, at a cursor, or at the end.

use std::sync::Arc;

use fs_people_core::{ListFilter, Result};

use crate::pages::{PersonPage, PersonPages};

/// A walk over the pages of one filter. Dropping it forgets the place; a new filter is a new walk.
pub struct PageWalk {
    _pages: Arc<dyn PersonPages>,
    _filter: ListFilter,
}

impl PageWalk {
    /// A walk at the start of the list `filter` selects.
    #[must_use]
    pub fn new(pages: Arc<dyn PersonPages>, filter: ListFilter) -> Self {
        Self {
            _pages: pages,
            _filter: filter,
        }
    }

    /// The next page, or `None` after the last. After an error the walk stays where it was.
    ///
    /// # Errors
    /// As [`PersonPages::first`] and [`PersonPages::next`].
    #[allow(clippy::unused_async)]
    pub async fn next_page(&mut self) -> Result<Option<PersonPage>> {
        todo!("step 4: walk")
    }
}
