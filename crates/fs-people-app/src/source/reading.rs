//! `Reading`: where "show more" stands in the list of one search term.
//!
//! Pattern: Iterator with State — a [`PageWalk`] (the cursor) plus the part of the last page that was
//! not yet shown (`carry`) and how many rows were shown (`served`). The workbench asks by
//! `offset`/`limit`; a request that starts exactly at `served` continues from the cursor, any other
//! one starts the walk again and skips (`DESIGN.md`, limits).

use std::collections::VecDeque;
use std::sync::Arc;

use fs_people_client::{PageWalk, PersonPages};
use fs_people_core::{ListFilter, Result};
use fs_store::StoredObject;

/// The filter of a search term; a blank term selects everybody.
fn filter_of(term: &str) -> ListFilter {
    ListFilter {
        term: (!term.trim().is_empty()).then(|| term.to_string()),
        ..ListFilter::default()
    }
}

/// The reading of one term.
pub(crate) struct Reading {
    pub(crate) term: String,
    pages: Arc<dyn PersonPages>,
    walk: PageWalk,
    carry: VecDeque<StoredObject>,
    served: usize,
}

impl Reading {
    /// At the start of the list of `term`.
    pub(crate) fn new(pages: Arc<dyn PersonPages>, term: &str) -> Self {
        Self {
            term: term.to_string(),
            walk: PageWalk::new(pages.clone(), filter_of(term)),
            pages,
            carry: VecDeque::new(),
            served: 0,
        }
    }

    /// The next row, reading a page only when the last one is used up.
    async fn pop(&mut self) -> Result<Option<StoredObject>> {
        while self.carry.is_empty() {
            match self.walk.next_page().await? {
                Some(page) => self.carry.extend(page.items),
                None => return Ok(None),
            }
        }
        Ok(self.carry.pop_front())
    }

    /// `limit` rows from `offset`. Continues from the cursor when `offset` is where the last call
    /// ended; otherwise starts again and skips `offset` rows.
    pub(crate) async fn take(&mut self, offset: usize, limit: usize) -> Result<Vec<StoredObject>> {
        if offset == 0 || offset != self.served {
            *self = Self::new(self.pages.clone(), &self.term.clone());
            while self.served < offset && self.pop().await?.is_some() {
                self.served += 1;
            }
            if self.served < offset {
                return Ok(Vec::new());
            }
        }
        let mut rows = Vec::new();
        while rows.len() < limit {
            let Some(row) = self.pop().await? else { break };
            rows.push(row);
        }
        self.served += rows.len();
        Ok(rows)
    }
}
