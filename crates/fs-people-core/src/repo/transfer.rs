//! Import and export through the format traits.
//!
//! Pattern: Template method — `import` is the fixed flow (parse -> skip known `uid` -> validate
//! and store -> report); the parsing step is the [`Importer`] strategy. `import_vcard` and friends are
//! thin conveniences for the shipped formats.

use fs_objects::ObjectId;

use super::handle::PeopleRepo;
use super::types::ImportReport;
use crate::error::{Error, Result};
use crate::format::{Exporter, Importer, JsContactFormat, VCardFormat};

impl PeopleRepo {
    /// Import the cards `importer` finds in `text`. Cards whose `uid` already exists, in any spelling,
    /// are skipped, never overwritten; the importer's original text (ADR 0030) is kept per card. A
    /// card with a sensitive marking that names no category is kept as it came, treated as sensitive
    /// and named in `unclear`: an import must not lose a record.
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
            let uid = imp.card.uid.clone();
            let name = format!("{uid}: {}", imp.card.display_name());
            let unclear = imp.card.is_sensitive() && !imp.card.unclear_sensitive_paths().is_empty();
            match self.get_by_uid(&uid).await {
                Ok(Some(_)) => {
                    report.skipped.push(uid);
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    report.failed.push(format!("{name}: {e}"));
                    continue;
                }
            }
            // Whatever stops this card, the report says so and the others of the file still count.
            match self.create_inner(imp.card, imp.original).await {
                Ok(o) => {
                    report.created.push(o.envelope.id);
                    if unclear {
                        report.unclear.push(name);
                    }
                }
                Err(e) => report.failed.push(format!("{name}: {e}")),
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
