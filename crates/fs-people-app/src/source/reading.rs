//! `Reading`: where "show more" stands in the list of one search term.
//!
//! Pattern: Iterator with State — a [`PageWalk`] (the cursor) plus the part of the last page that was
//! not yet shown (`carry`) and how many rows were shown (`served`). The workbench asks by
//! `offset`/`limit`; a request that starts at `served` or inside the rows just given continues from the cursor, any other
//! one starts the walk again and skips (`DESIGN.md`, limits).

use std::collections::VecDeque;
use std::sync::Arc;

use fs_people_client::{PageWalk, PersonPages};
use fs_people_core::{ListFilter, Result};
use fs_objects::StoredObject;

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
    /// The rows of the last call, which end at `served`: the workbench asks for one row more than
    /// it shows and asks on from what it shows, so the next call may start inside them.
    kept: Vec<StoredObject>,
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
            kept: Vec::new(),
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

    /// Put back the last `back` rows delivered, so they are delivered again.
    fn rewind(&mut self, back: usize) {
        let from = self.kept.len() - back;
        for row in self.kept.drain(from..).rev() {
            self.carry.push_front(row);
        }
        self.served -= back;
    }

    /// `limit` rows from `offset`. Continues from the cursor when `offset` is where the last call
    /// ended, or inside the rows the last call delivered (they are given again); from 0 or anywhere
    /// else it starts again (a reload must see changes) and skips.
    pub(crate) async fn take(&mut self, offset: usize, limit: usize) -> Result<Vec<StoredObject>> {
        let back = self.served.saturating_sub(offset);
        if offset > 0 && back > 0 && back <= self.kept.len() {
            self.rewind(back);
        } else if offset == 0 || offset != self.served {
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
        if !rows.is_empty() {
            self.kept.clone_from(&rows);
        }
        Ok(rows)
    }
}
