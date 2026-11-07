//! Saving is compare-and-set: a change that lands between the adapter's read and its write is not
//! overwritten. The server, store and client are the real ones; only `People::get` is decorated, to
//! let someone else write at the moment between read and write.
//!
//! Pattern: Decorator over the real client, in an integration test.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use common::{World, person};
use fs_components_workbenches::people::PeopleSource;
use fs_people_app::PeopleAdapter;
use fs_people_client::RpcPeople;
use fs_people_core::{Card, People, Result, card_of};
use fs_store::{ObjectId, StoredObject};

/// The real client whose `get` lets another writer in right after it has read.
struct Interfering {
    inner: Arc<RpcPeople>,
}

#[async_trait]
impl People for Interfering {
    fn capabilities(&self) -> fs_interfaces::Capabilities {
        self.inner.capabilities()
    }
    async fn list(&self, filter: &fs_people_core::ListFilter) -> Result<Vec<StoredObject>> {
        self.inner.list(filter).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        let read = self.inner.get(id).await?;
        if let Some(object) = &read {
            let mut theirs = card_of(object)?;
            theirs.add_phone("+49 30 7");
            self.inner.update(id, theirs).await?;
        }
        Ok(read)
    }
    async fn update_checked(
        &self,
        id: ObjectId,
        expected: fs_people_core::Revision,
        card: Card,
    ) -> Result<StoredObject> {
        self.inner.update_checked(id, expected, card).await
    }
}

#[test]
fn a_write_between_read_and_save_is_a_conflict_and_nothing_is_lost() {
    let w = World::in_memory();
    let key = w.add(person("Ada", "ada@x.org"));
    let source = PeopleAdapter::new(
        Arc::new(Interfering {
            inner: w.people.clone(),
        }),
        w.people.clone(),
        w.pages.clone(),
        w.people.clone(),
        w.runtime.handle().clone(),
    );
    let mut draft = source.draft(&key).unwrap();
    let name = draft
        .groups
        .iter_mut()
        .flat_map(|g| g.fields.iter_mut())
        .find(|f| f.key == "name/full")
        .unwrap();
    name.value = "Mine".into();
    let outcome = std::thread::scope(|s| s.spawn(|| source.save(&key, &draft)).join().unwrap());
    let error = outcome.expect_err("their write came in between");
    assert!(error.message.contains("Someone else"), "{}", error.message);
    let card = w.card(&key);
    assert_eq!(card.display_name(), "Ada", "my change was not written");
    assert_eq!(card.phones.len(), 1, "their change is still there");
}
