//! `PageWalk`: the pages of a filter, one `next_page` after the other.
//!
//! Pattern: Iterator (async, so not `std::iter::Iterator`) over [`PersonPages`], with State: the
//! walk is at the start, at a cursor, or at the end.

use std::sync::Arc;

use fs_people_core::{ListFilter, Result};

use crate::pages::{Cursor, PersonPage, PersonPages};

/// Where the walk stands.
enum Place {
    Start,
    At(Cursor),
    End,
}

/// A walk over the pages of one filter. Dropping it forgets the place; a new filter is a new walk.
pub struct PageWalk {
    pages: Arc<dyn PersonPages>,
    filter: ListFilter,
    place: Place,
}

impl PageWalk {
    /// A walk at the start of the list `filter` selects.
    #[must_use]
    pub fn new(pages: Arc<dyn PersonPages>, filter: ListFilter) -> Self {
        Self {
            pages,
            filter,
            place: Place::Start,
        }
    }

    /// The next page, or `None` after the last. After an error the walk stays where it was.
    ///
    /// # Errors
    /// As [`PersonPages::first`] and [`PersonPages::next`].
    pub async fn next_page(&mut self) -> Result<Option<PersonPage>> {
        let page = match &self.place {
            Place::End => return Ok(None),
            Place::Start => self.pages.first(&self.filter).await?,
            Place::At(cursor) => self.pages.next(&self.filter, cursor).await?,
        };
        self.place = page.next.clone().map_or(Place::End, Place::At);
        Ok(Some(page))
    }
}
