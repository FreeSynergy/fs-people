//! Import and export through the format traits.
//!
//! Pattern: Template method — `import` is the fixed flow (parse -> skip known `uid` -> validate
//! and store -> report); the parsing step is the [`Importer`] strategy. `import_vcard` and friends are
//! thin conveniences for the shipped formats.

use fs_store::ObjectId;

use super::handle::PeopleRepo;
use super::types::ImportReport;
use crate::error::{Error, Result};
use crate::format::{Exporter, Importer, JsContactFormat, VCardFormat};

impl PeopleRepo {
    /// Import the cards `importer` finds in `text`. Cards whose `uid` already exists are skipped,
    /// never overwritten; the importer's original text (ADR 0030) is kept per card.
    pub async fn import(&self, importer: &dyn Importer, text: &str) -> Result<ImportReport> {
        let mut report = ImportReport::default();
        for item in importer.import(text)? {
            let imp = match item {
                Ok(imp) => imp,
                Err(e) => {
                    report.failed.push(e.to_string());
                    continue;
                }
            };
            if self.get_by_uid(&imp.card.uid).await?.is_some() {
                report.skipped.push(imp.card.uid);
                continue;
            }
            let uid = imp.card.uid.clone();
            match self.create_inner(imp.card, imp.original).await {
                Ok(o) => report.created.push(o.envelope.id),
                Err(e @ Error::Invalid(_)) => report.failed.push(format!("{uid}: {e}")),
                Err(e) => return Err(e),
            }
        }
        Ok(report)
    }

    /// Import a `.vcf` text (one or many vCards). Each block is kept as the original (ADR 0030).
    pub async fn import_vcard(&self, text: &str) -> Result<ImportReport> {
        self.import(&VCardFormat, text).await
    }

    /// Import JSContact JSON (one card or an array), e.g. the demo data.
    pub async fn import_jscontact(&self, json_text: &str) -> Result<ImportReport> {
        self.import(&JsContactFormat, json_text).await
    }

    /// The card as text of the format `exporter` objects.
    pub async fn export(&self, exporter: &dyn Exporter, id: ObjectId) -> Result<String> {
        let card = self.get_card(id).await?.ok_or(Error::NotFound(id))?;
        exporter.export(&card)
    }

    /// The card as vCard 4.0 text.
    pub async fn export_vcard(&self, id: ObjectId) -> Result<String> {
        self.export(&VCardFormat, id).await
    }
}
