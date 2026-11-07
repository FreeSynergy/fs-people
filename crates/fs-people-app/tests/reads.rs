//! How often the app asks the server for the list: an export reads it once, not once per 500.
//!
//! Pattern: Decorator — a counting wrapper around the real client; it delegates every call.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use common::{Scratch, World, person};
use fs_interfaces::Capabilities;
use fs_people_core::{Exporter, ListFilter, People, ReleasedExport, Result};
use fs_store::{ObjectId, StoredObject};

struct Counting {
    inner: Arc<dyn People>,
    lists: AtomicUsize,
}

#[async_trait]
impl People for Counting {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        self.inner.get(id).await
    }
    async fn list(&self, filter: &ListFilter) -> Result<Vec<StoredObject>> {
        self.lists.fetch_add(1, Ordering::SeqCst);
        self.inner.list(filter).await
    }
    async fn export_released(
        &self,
        exporter: &dyn Exporter,
        id: ObjectId,
        released: &[String],
    ) -> Result<ReleasedExport> {
        self.inner.export_released(exporter, id, released).await
    }
}

#[test]
fn an_export_of_more_than_one_chunk_reads_the_list_once() {
    let (w, dir) = (World::in_memory(), Scratch::new());
    for i in 0..520 {
        w.add(person(&format!("P{i:03}"), &format!("p{i}@x.org")));
    }
    let counting = Counting {
        inner: w.people.clone(),
        lists: AtomicUsize::new(0),
    };
    let out = dir.file("all.vcf");
    let report = fs_people_app::export_all(&counting, w.runtime.handle(), &out, &[]).unwrap();
    assert_eq!(report.written, 520);
    assert_eq!(
        counting.lists.load(Ordering::SeqCst),
        1,
        "one read of the list"
    );
}
