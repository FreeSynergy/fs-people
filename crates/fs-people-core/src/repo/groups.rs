//! Groups / folders / mailing lists (ADR 0037).
//!
//! Pattern: Repository (composite) — a group is a normal card of `kind: group`; membership is
//! the `members` map, so no second object kind is needed.

use fs_store::{ObjectId, Query, StoredObject};

use super::consts::PROFILE_TYPE;
use super::envelope::card_of;
use super::handle::PeopleRepo;
use crate::card::Card;
use crate::error::{Error, Result};
use crate::index::normalise_uid;

impl PeopleRepo {
    /// A group (folder, mailing list): a card with `kind: group` and `members`.
    pub async fn create_group(&self, name: &str) -> Result<StoredObject> {
        self.create(Card::group(name)).await
    }

    /// Add `member` to `group`.
    pub async fn add_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        let mut g = self.group_card(group).await?;
        if group == member {
            return Err(Error::Conflict("a group cannot contain itself".into()));
        }
        let m = self
            .get_card(member)
            .await?
            .ok_or(Error::NotFound(member))?;
        g.members.insert(m.uid, true);
        self.update(group, g).await
    }

    /// Remove `member` from `group`, by its `uid` in any spelling. A member whose card is gone can be
    /// removed too, if its `uid` was its id.
    pub async fn remove_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        let mut g = self.group_card(group).await?;
        let card = self.get_card(member).await?;
        let known = card.is_some();
        let uid = card.map_or_else(|| member.to_string(), |m| m.uid);
        let before = g.members.len();
        retain_others(&mut g, &uid);
        if !known && g.members.len() == before {
            return Err(Error::NotFound(member));
        }
        self.update(group, g).await
    }

    /// Resolved members of a group (members whose card no longer exists are left out).
    pub async fn members(&self, group: ObjectId) -> Result<Vec<StoredObject>> {
        let g = self.group_card(group).await?;
        let mut out = vec![];
        for uid in g.members.keys() {
            if let Some(o) = self.get_by_uid(uid).await? {
                out.push(o);
            }
        }
        Ok(out)
    }

    /// Take this `uid` off every group card, so it cannot come back with a new card of that `uid`.
    pub(super) async fn remove_from_all_groups(&self, uid: &str) -> Result<()> {
        let q = Query::new()
            .of_type(PROFILE_TYPE)
            .with_index("kind", "group");
        for stored in self.reads.query(&q).await? {
            let mut g = card_of(&stored)?;
            let before = g.members.len();
            retain_others(&mut g, uid);
            if g.members.len() != before {
                self.update(stored.envelope.id, g).await?;
            }
        }
        Ok(())
    }

    async fn group_card(&self, group: ObjectId) -> Result<Card> {
        let g = self.get_card(group).await?.ok_or(Error::NotFound(group))?;
        if !g.is_group() {
            return Err(Error::Conflict(format!("{group} is not a group")));
        }
        Ok(g)
    }

    /// Groups that contain `member`.
    pub async fn groups_of(&self, member: ObjectId) -> Result<Vec<StoredObject>> {
        let m = self
            .get_card(member)
            .await?
            .ok_or(Error::NotFound(member))?;
        let q = Query::new()
            .of_type(PROFILE_TYPE)
            .with_index("kind", "group")
            .with_index("member", m.uid);
        Ok(self.reads.query(&q).await?)
    }
}

fn retain_others(group: &mut Card, uid: &str) {
    let key = normalise_uid(uid);
    group
        .members
        .retain(|member, _| normalise_uid(member) != key);
}
