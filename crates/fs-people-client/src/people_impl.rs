//! `People` over the socket: the methods the app uses.
//!
//! Pattern: Adapter — each method is one call of `Persons` (`update_checked` is `updateIf`). Every other method of the trait is not
//! overridden, so it answers the explicit `Unsupported` of the provider rule (ADR 0045); `search`
//! is `list` with a term, as the contract has no search of its own.

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_people_core::{
    Card, Exporter, ImportReport, ListFilter, People, ReleasedExport, Result, Revision,
};
use fs_objects::{ObjectId, StoredObject};

use crate::RpcPeople;
use crate::calls::lists;
use crate::calls::persons::{self, card_text};
use crate::pages::{Cursor, PersonPage, PersonPages};
use crate::revised::{Revised, RevisedPeople};

const VCARD: &str = "text/vcard";

#[async_trait]
impl People for RpcPeople {
    fn capabilities(&self) -> Capabilities {
        self.report()
    }

    async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        self.call(move |r| async move { persons::get(&r.persons, id).await })
            .await
    }

    async fn list(&self, filter: &ListFilter) -> Result<Vec<StoredObject>> {
        let filter = filter.clone();
        self.call(move |r| async move { lists::list(&r.persons, &filter).await })
            .await
    }

    async fn create(&self, card: Card) -> Result<StoredObject> {
        let card = card_text(&card);
        self.call(move |r| async move { persons::create(&r.persons, card).await })
            .await
    }

    async fn update(&self, id: ObjectId, card: Card) -> Result<StoredObject> {
        let card = card_text(&card);
        self.call(move |r| async move { persons::update(&r.persons, id, card).await })
            .await
    }

    async fn update_checked(
        &self,
        id: ObjectId,
        expected: Revision,
        card: Card,
    ) -> Result<StoredObject> {
        let card = card_text(&card);
        self.call(move |r| async move { persons::update_if(&r.persons, id, expected, card).await })
            .await
    }

    async fn delete(&self, id: ObjectId) -> Result<()> {
        self.call(move |r| async move { persons::delete(&r.persons, id).await })
            .await
    }

    async fn search(&self, term: &str) -> Result<Vec<StoredObject>> {
        let filter = ListFilter {
            term: Some(term.to_string()),
            ..ListFilter::default()
        };
        self.list(&filter).await
    }

    async fn import_vcard(&self, text: &str) -> Result<ImportReport> {
        let text = text.to_string();
        self.call(move |r| async move { persons::import(&r.persons, VCARD, text).await })
            .await
    }

    async fn export_released(
        &self,
        exporter: &dyn Exporter,
        id: ObjectId,
        released: &[String],
    ) -> Result<ReleasedExport> {
        let media_type = exporter.output_media_type();
        let released = released.to_vec();
        self.call(
            move |r| async move { persons::export(&r.persons, id, media_type, released).await },
        )
        .await
    }
}

#[async_trait]
impl PersonPages for RpcPeople {
    async fn first(&self, filter: &ListFilter) -> Result<PersonPage> {
        let filter = filter.clone();
        self.call(move |r| async move { lists::page(&r.persons, &filter, None).await })
            .await
    }

    async fn next(&self, filter: &ListFilter, cursor: &Cursor) -> Result<PersonPage> {
        let (filter, cursor) = (filter.clone(), cursor.clone());
        self.call(move |r| async move { lists::page(&r.persons, &filter, Some(&cursor)).await })
            .await
    }
}

#[async_trait]
impl RevisedPeople for RpcPeople {
    async fn get_revised(&self, id: ObjectId) -> Result<Option<Revised>> {
        self.call(move |r| async move { persons::get_revised(&r.persons, id).await })
            .await
    }
}
