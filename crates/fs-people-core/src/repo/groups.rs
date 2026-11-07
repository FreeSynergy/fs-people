//! Groups / folders / mailing lists (ADR 0037).
//!
//! Pattern: Repository (composite) — a group is a normal card of `kind: group`; membership is
//! the `members` map, so no second object kind is needed.

use fs_objects::{ObjectId, Query, StoredObject};

use super::consts::PROFILE_TYPE;
use super::handle::PeopleRepo;
use crate::card::Card;
use crate::error::{Error, Result};

impl PeopleRepo {
    /// A group (folder, mailing list): a card with `kind: group` and `members`.
    pub async fn create_group(&self, name: &str) -> Result<StoredObject> {
        self.create(Card::group(name)).await
    }

    /// Add `member` to `group`.
    pub async fn add_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        let mut g = self.get_card(group).await?.ok_or(Error::NotFound(group))?;
        if !g.is_group() {
            return Err(Error::Conflict(format!("{group} is not a group")));
        }
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

    /// Remove `member` from `group`.
    pub async fn remove_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        let mut g = self.get_card(group).await?.ok_or(Error::NotFound(group))?;
        let m = self
            .get_card(member)
            .await?
            .ok_or(Error::NotFound(member))?;
        g.members.remove(&m.uid);
        self.update(group, g).await
    }

    /// Resolved members of a group (members whose card no longer exists are left out).
    pub async fn members(&self, group: ObjectId) -> Result<Vec<StoredObject>> {
        let g = self.get_card(group).await?.ok_or(Error::NotFound(group))?;
        let mut out = vec![];
        for uid in g.members.keys() {
            if let Some(o) = self.get_by_uid(uid).await? {
                out.push(o);
            }
        }
        Ok(out)
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
