//! vCard files in and out, by a path from the command line.
//!
//! Pattern: Template method over the contract — fixed flow: read, import or export per person,
//! report. Nothing is hidden: what failed or was left out is in the report.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;

use fs_people_core::{ListFilter, People};
use tokio::runtime::Handle;

use crate::source::reason_text;

/// Asked of the core at a time while exporting.
const EXPORT_PAGE: usize = 500;

/// What an import did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportSummary {
    /// People who were added.
    pub created: usize,
    /// People who were known already and were left as they are.
    pub skipped: usize,
    /// What could not be imported, with the reason.
    pub failed: Vec<String>,
    /// The import stopped here: the store refused a card (the people before it are in).
    pub stopped: Option<String>,
}

impl ImportSummary {
    /// Did all of it work?
    pub fn is_clean(&self) -> bool {
        self.failed.is_empty() && self.stopped.is_none()
    }

    /// In words, for the status line.
    pub fn message(&self) -> String {
        let mut text = format!("Imported {}, {} already known", self.created, self.skipped);
        if !self.failed.is_empty() {
            let _ = write!(
                text,
                ", {} failed: {}",
                self.failed.len(),
                self.failed.join("; ")
            );
        }
        if let Some(why) = &self.stopped {
            let _ = write!(text, ". The import stopped: {why}");
        }
        text
    }
}

/// Import the vCard file at `path`.
///
/// # Errors
/// A message for the person if the file cannot be read or holds no vCard.
pub fn import_file(
    people: &dyn People,
    runtime: &Handle,
    path: &Path,
) -> Result<ImportSummary, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if !text.to_ascii_uppercase().contains("BEGIN:VCARD") {
        return Err(format!("{} holds no vCard", path.display()));
    }
    let mut summary = ImportSummary::default();
    match runtime.block_on(people.import_vcard(&text)) {
        Ok(report) => {
            summary.created = report.created.len();
            summary.skipped = report.skipped.len();
            summary.failed = report.failed;
        }
        Err(e) => summary.stopped = Some(reason_text(&e)),
    }
    Ok(summary)
}

/// What an export did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportReport {
    /// People written.
    pub written: usize,
    /// People marked sensitive, left out (ADR 0029: they leave only by explicit release).
    pub left_out: Vec<String>,
    /// People who could not be expressed as vCard, with the reason.
    pub failed: Vec<String>,
}

impl ExportReport {
    /// In words.
    pub fn message(&self, path: &Path) -> String {
        let mut text = format!("Wrote {} to {}", self.written, path.display());
        if !self.left_out.is_empty() {
            let _ = write!(
                text,
                ". Left out as sensitive: {}",
                self.left_out.join(", ")
            );
        }
        if !self.failed.is_empty() {
            let _ = write!(text, ". Failed: {}", self.failed.join("; "));
        }
        text
    }
}

/// Write everyone to a new file at `path`. A file that exists is not replaced. Roles are never in
/// it, because they are not on the card.
///
/// # Errors
/// A message for the person if the list cannot be read or the file cannot be written.
pub fn export_all(
    people: &dyn People,
    runtime: &Handle,
    path: &Path,
) -> Result<ExportReport, String> {
    let mut report = ExportReport::default();
    let mut text = String::new();
    let mut offset = 0;
    loop {
        let filter = ListFilter {
            limit: Some(EXPORT_PAGE),
            offset,
            ..ListFilter::default()
        };
        let page = runtime
            .block_on(people.list(&filter))
            .map_err(|e| reason_text(&e))?;
        for object in &page {
            let name = &object.envelope.name;
            if !object.envelope.sensitive.is_empty() {
                report.left_out.push(name.clone());
                continue;
            }
            match runtime.block_on(people.export_vcard(object.envelope.id)) {
                Ok(card) => {
                    text.push_str(&card);
                    report.written += 1;
                }
                Err(e) => report.failed.push(format!("{name}: {e}")),
            }
        }
        offset += page.len();
        if page.len() < EXPORT_PAGE {
            break;
        }
    }
    let mut file = std::fs::File::create_new(path)
        .map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    file.write_all(text.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(report)
}
