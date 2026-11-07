//! What leaves the program with an explicit release, and only then (ADR 0029).
//!
//! Pattern: Specification + Value object — adding someone sensitive to a list is allowed when asked for,
//! as nothing links them automatically. Taking a sensitive card out of the program is not: it needs a
//! release per recipient, which the caller names by `uid`. Marked members of a list stay behind, and the
//! report names them. The card itself refuses: there is nothing to send without it.

use fs_objects::ObjectId;

use super::envelope::{card_of, is_marked};
use super::handle::PeopleRepo;
use crate::card::Card;
use crate::error::{Error, Result};
use crate::format::Exporter;
use crate::index::normalise_uid;
use crate::subject::{Kind, Subject};

/// A card as text, and who was left out of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleasedExport {
    /// The card in the exporter's format.
    pub text: String,
    /// Sensitive members of a list not taken along, as `uid: name`.
    pub withheld: Vec<String>,
}

impl PeopleRepo {
    /// The card `id` as text. A sensitive card goes only if its `uid` is in `released`, or the answer
    /// is [`Error::NotReleased`]. Of a list, sensitive members not in `released` are left out.
    pub async fn export_released(
        &self,
        exporter: &dyn Exporter,
        id: ObjectId,
        released: &[String],
    ) -> Result<ReleasedExport> {
        let stored = self.get(id).await?.ok_or(Error::NotFound(id))?;
        let mut card = card_of(&stored)?;
        let released: Vec<String> = released.iter().map(|uid| normalise_uid(uid)).collect();
        if is_marked(&stored) && !released.contains(&normalise_uid(&card.uid)) {
            let who = format!("{}: {}", card.uid, stored.envelope.name);
            return Err(Error::NotReleased(subject_of(id, &card), who));
        }
        let mut withheld = Vec::new();
        for (uid, name) in self.sensitive_members_of(&card).await? {
            if released.contains(&normalise_uid(&uid)) {
                continue;
            }
            card.members.remove(&uid);
            withheld.push(name);
        }
        Ok(ReleasedExport {
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
                && is_marked(&member)
            {
                found.push((uid.clone(), format!("{uid}: {}", member.envelope.name)));
            }
        }
        Ok(found)
    }
}

/// The card `id` as the subject of an error: a list or a person.
pub(super) fn subject_of(id: ObjectId, card: &Card) -> Subject {
    let kind = if card.is_group() {
        Kind::List
    } else {
        Kind::Person
    };
    Subject::new(kind, id)
}
