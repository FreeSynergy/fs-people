//! The counting reader of pages for the tests.
//!
//! Pattern: Decorator over the real client.
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use fs_people_client::{Cursor, PersonPage, PersonPages, RpcPeople};
use fs_people_core::{ListFilter, Result as PeopleResult};

/// `PersonPages` over the real client that counts the pages it reads.
pub struct Counting {
    inner: Arc<RpcPeople>,
    firsts: AtomicUsize,
    nexts: AtomicUsize,
}

impl Counting {
    pub fn over(inner: Arc<RpcPeople>) -> Self {
        Self {
            inner,
            firsts: AtomicUsize::new(0),
            nexts: AtomicUsize::new(0),
        }
    }

    /// (first pages read, continued pages read) so far.
    pub fn reads(&self) -> (usize, usize) {
        (
            self.firsts.load(Ordering::SeqCst),
            self.nexts.load(Ordering::SeqCst),
        )
    }
}

#[async_trait::async_trait]
impl PersonPages for Counting {
    async fn first(&self, filter: &ListFilter) -> PeopleResult<PersonPage> {
        self.firsts.fetch_add(1, Ordering::SeqCst);
        self.inner.first(filter).await
    }

    async fn next(&self, filter: &ListFilter, cursor: &Cursor) -> PeopleResult<PersonPage> {
        self.nexts.fetch_add(1, Ordering::SeqCst);
        self.inner.next(filter, cursor).await
    }
}
