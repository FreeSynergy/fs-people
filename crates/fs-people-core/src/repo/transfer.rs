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
use crate::validate::validate;

/// How much text a report keeps per list, however large the file.
const MAX_REPORT_TEXT_BYTES: usize = 8 * 1024 * 1024;
/// Items between two pauses of an import, so the others on this thread get their turn.
const YIELD_EVERY: usize = 256;

impl PeopleRepo {
    /// Import the cards `importer` finds in `text`. Cards whose `uid` already exists, in any spelling,
    /// are skipped, never overwritten; the importer's original text (ADR 0030) is kept per card. A
    /// card with a sensitive marking that names no category is kept as it came, treated as sensitive
    /// and named in `unclear`: an import must not lose a record.
    ///
    /// Each card takes its own turn among the writes, so a large file holds nobody up for long. What
    /// fails is known before the store is asked. The texts of the report are bounded: what does not
    /// fit is counted in a last entry.
    pub async fn import(&self, importer: &dyn Importer, text: &str) -> Result<ImportReport> {
        let (mut created, mut skipped, mut failed, mut unclear_ones) = (
            Vec::new(),
            Bounded::default(),
            Bounded::default(),
            Bounded::default(),
        );
        for (i, item) in importer.import_each(text)?.enumerate() {
            if i % YIELD_EVERY == 0 {
                tokio::task::yield_now().await; // a file of failures never waits for the store
            }
            let imp = match item {
                Ok(imp) => imp,
                Err(e) => {
                    failed.push(e.to_string());
                    continue;
                }
            };
            let uid = imp.card.uid.clone();
            let name = format!("{uid}: {}", imp.card.display_name());
            if let Err(e) = validate(&imp.card) {
                failed.push(format!("{name}: {e}"));
                continue;
            }
            let unclear = !imp.card.unclear_sensitive_paths().is_empty();
            let _turn = self.writes.lock().await;
            match self.get_by_uid(&uid).await {
                Ok(Some(_)) => {
                    skipped.push(uid);
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    failed.push(format!("{name}: {e}"));
                    continue;
                }
            }
            // Whatever stops this card, the report says so and the others of the file still count.
            match self.create_inner(imp.card, imp.original).await {
                Ok(o) => {
                    created.push(o.envelope.id);
                    if unclear {
                        unclear_ones.push(name);
                    }
                }
                Err(e) => failed.push(format!("{name}: {e}")),
            }
        }
        Ok(ImportReport {
            created,
            skipped: skipped.finish(),
            failed: failed.finish(),
            unclear: unclear_ones.finish(),
        })
    }

    /// Import a `.vcf` text (one or many vCards). Each block is kept as the original (ADR 0030).
    pub async fn import_vcard(&self, text: &str) -> Result<ImportReport> {
        self.import(&VCardFormat, text).await
    }

    /// Import JSContact JSON (one card or an array), e.g. the demo data.
    pub async fn import_jscontact(&self, json_text: &str) -> Result<ImportReport> {
        self.import(&JsContactFormat, json_text).await
    }

    /// The card as text of the format `exporter` objects. A list with sensitive members is refused
    /// here, by name: `export_list` leaves them out or takes them along on explicit release.
    pub async fn export(&self, exporter: &dyn Exporter, id: ObjectId) -> Result<String> {
        let card = self.get_card(id).await?.ok_or(Error::NotFound(id))?;
        let withheld = self.sensitive_members_of(&card).await?;
        if !withheld.is_empty() {
            let names = withheld.into_iter().map(|(_, name)| name).collect();
            return Err(Error::SensitiveMembers(names));
        }
        exporter.export(&card)
    }

    /// The card as vCard 4.0 text.
    pub async fn export_vcard(&self, id: ObjectId) -> Result<String> {
        self.export(&VCardFormat, id).await
    }
}

/// Texts of one list of a report, up to [`MAX_REPORT_TEXT_BYTES`].
#[derive(Default)]
struct Bounded {
    items: Vec<String>,
    bytes: usize,
    more: usize,
}

impl Bounded {
    fn push(&mut self, text: String) {
        self.bytes = self.bytes.saturating_add(text.len());
        if self.bytes > MAX_REPORT_TEXT_BYTES {
            self.more += 1;
        } else {
            self.items.push(text);
        }
    }

    fn finish(mut self) -> Vec<String> {
        if self.more > 0 {
            self.items.push(format!("{} more", self.more));
        }
        self.items
    }
}
