//! Deleting a person, and everything that belongs to them.
//!
//! Pattern: Template method — the fixed order of the steps, as the store has no transaction over several
//! objects: off every list, as only the card knows its `uid`; the card; links and roles. Each failure
//! says how far it got, and the same call again finishes the job.

use fs_objects::ObjectId;

use super::handle::PeopleRepo;
use crate::error::{Error, Result};
use crate::feed::PersonChange;

impl PeopleRepo {
    /// Delete a card (undoable through the store). It leaves every list, and its role assignments,
    /// the same-person links that mention it and, for a group card, the assignments scoped to it are
    /// removed too.
    pub async fn delete(&self, id: ObjectId) -> Result<()> {
        let found = self.get_card(id).await?;
        let card_there = found.is_some();
        if let Some(card) = found {
            let (lists, pruned) = self.remove_from_all_groups(&card.uid).await;
            let gone = match pruned {
                Ok(()) => self
                    .objects
                    .delete(id, &self.actor)
                    .await
                    .map_err(Error::from),
                Err(e) => Err(e),
            };
            if let Err(e) = gone {
                if lists == 0 {
                    return Err(e);
                }
                return Err(Error::PartlyDeleted(
                    id,
                    format!("the person is still there, but off {lists} lists ({e})"),
                ));
            }
            self.feed.notify(&PersonChange::Deleted(id));
        }
        let cleaned = self.remove_what_belongs_to(id, card_there).await;
        match cleaned {
            Err(e) if card_there && !matches!(e, Error::NotFound(_)) => Err(Error::PartlyDeleted(
                id,
                format!(
                    "the person is deleted, but links or roles that belonged to them remain ({e})"
                ),
            )),
            other => other,
        }
    }

    /// The links and roles of `id`. `NotFound` if there was neither a card nor anything of this.
    async fn remove_what_belongs_to(&self, id: ObjectId, card_was_there: bool) -> Result<()> {
        let links = self.relationships_of(id).await?;
        let rejections = self.rejections_of(id).await?;
        let roles = self.roles_of(id).await?;
        let scoped = self.assignments_scoped_to(&self.iri(id)).await?;
        if !card_was_there && links.is_empty() && roles.is_empty() && scoped.is_empty() {
            return Err(Error::NotFound(id));
        }
        for link in links.into_iter().chain(rejections) {
            self.objects.delete(link.envelope.id, &self.actor).await?;
        }
        for held in scoped {
            self.objects.delete(held, &self.actor).await?;
        }
        self.delete_assignments_of(id).await
    }
}
