//! Exporting a list that has sensitive members (ADR 0029).
//!
//! Pattern: Specification + Value object — adding someone sensitive to a list is allowed when asked for,
//! as nothing links them automatically. Taking them along out of the program is not: an export or share
//! of a list treats them as an export of their card would, and the report names who stayed behind.

use fs_store::ObjectId;

use super::handle::PeopleRepo;
use crate::card::Card;
use crate::error::{Error, Result};
use crate::format::Exporter;
use crate::index::normalise_uid;

/// A list as text, and who was left out of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListExport {
    /// The list in the exporter's format.
    pub text: String,
    /// Sensitive members not taken along, as `uid: name`.
    pub withheld: Vec<String>,
}

impl PeopleRepo {
    /// The list `id` as text, without its sensitive members; those whose `uid` is in `released` go
    /// along, as they were explicitly released. Any other card is exported as it is.
    pub async fn export_list(
        &self,
        exporter: &dyn Exporter,
        id: ObjectId,
        released: &[String],
    ) -> Result<ListExport> {
        let mut card = self.get_card(id).await?.ok_or(Error::NotFound(id))?;
        let released: Vec<String> = released.iter().map(|uid| normalise_uid(uid)).collect();
        let mut withheld = Vec::new();
        for (uid, name) in self.sensitive_members_of(&card).await? {
            if released.contains(&normalise_uid(&uid)) {
                continue;
            }
            card.members.remove(&uid);
            withheld.push(name);
        }
        Ok(ListExport {
            text: exporter.export(&card)?,
            withheld,
        })
    }

    /// The members of a list that are marked sensitive: their `uid` as the list has it, and a name
    /// for a report.
    pub(super) async fn sensitive_members_of(&self, card: &Card) -> Result<Vec<(String, String)>> {
        let mut found = Vec::new();
        if !card.is_group() {
            return Ok(found);
        }
        for uid in card.members.keys() {
            if let Some(member) = self.get_by_uid(uid).await?
                && !member.envelope.sensitive.is_empty()
            {
                found.push((uid.clone(), format!("{uid}: {}", member.envelope.name)));
            }
        }
        Ok(found)
    }
}
