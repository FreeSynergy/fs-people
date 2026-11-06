//! Groups / folders / mailing lists (ADR 0037).
//!
//! Pattern: Repository (composite) — a group is a normal card of `kind: group`; membership is
//! the `members` map, so no second object kind is needed.

use fs_store::{Cond, ObjectId, Op, Query, StoredObject};
use serde_json::Value;

use super::consts::PROFILE_TYPE;
use super::handle::PeopleRepo;
use crate::card::Card;
use crate::clock::now_utc;
use crate::error::{Error, Result};
use crate::feed::PersonChange;
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

    /// Take this `uid` off every group card, so it cannot come back with a new card of that `uid`:
    /// from what is stored as it is, so a list that does not read as a card cannot block anyone's
    /// delete. Answers how many lists changed, also next to an error.
    pub(super) async fn remove_from_all_groups(&self, uid: &str) -> (usize, Result<()>) {
        let key = normalise_uid(uid);
        let q = Query::new()
            .of_type(PROFILE_TYPE)
            .with_index("kind", "group");
        let found = match self.reads.query(&q).await {
            Ok(found) => found,
            Err(e) => return (0, Err(e.into())),
        };
        let mut changed = 0;
        for mut stored in found {
            let Some(members) = stored
                .content
                .get_mut("members")
                .and_then(Value::as_object_mut)
            else {
                continue;
            };
            let before = members.len();
            members.retain(|member, _| normalise_uid(member) != key);
            if members.len() == before {
                continue;
            }
            let left: Vec<String> = members.keys().map(|m| normalise_uid(m)).collect();
            if left.is_empty() {
                stored
                    .content
                    .as_object_mut()
                    .map(|card| card.remove("members"));
            }
            if let Some(card) = stored.content.as_object_mut() {
                card.insert("updated".into(), now_utc().into());
            }
            stored.index.insert("member".into(), left);
            let id = stored.envelope.id;
            if let Err(e) = self.objects.put(stored, &self.actor).await {
                return (changed, Err(e.into()));
            }
            self.feed.notify(&PersonChange::Updated(id));
            changed += 1;
        }
        (changed, Ok(()))
    }

    async fn group_card(&self, group: ObjectId) -> Result<Card> {
        let g = self.get_card(group).await?.ok_or(Error::NotFound(group))?;
        if !g.is_group() {
            return Err(Error::Conflict(format!("{group} is not a group")));
        }
        Ok(g)
    }

    /// Groups that contain `member`, in any spelling of its `uid`. Lists stored before the index
    /// held the one spelling are found by a search, and what it finds is checked.
    pub async fn groups_of(&self, member: ObjectId) -> Result<Vec<StoredObject>> {
        let m = self
            .get_card(member)
            .await?
            .ok_or(Error::NotFound(member))?;
        let key = normalise_uid(&m.uid);
        let has = |op: Op, value: &str| Cond::Index {
            key: "member".into(),
            op,
            value: value.into(),
        };
        let mut q = Query::new()
            .of_type(PROFILE_TYPE)
            .with_index("kind", "group")
            .or(has(Op::Eq, &key))
            .or(has(Op::Eq, &m.uid));
        if key.len() == 36 {
            q = q
                .or(has(Op::Contains, &key))
                .or(has(Op::Contains, &key.replace('-', "")));
        }
        let holds = |o: &StoredObject| {
            let members = o.content.get("members").and_then(Value::as_object);
            members.is_some_and(|all| all.keys().any(|k| normalise_uid(k) == key))
        };
        let mut found = self.reads.query(&q).await?;
        found.retain(holds);
        Ok(found)
    }
}

fn retain_others(group: &mut Card, uid: &str) {
    let key = normalise_uid(uid);
    group
        .members
        .retain(|member, _| normalise_uid(member) != key);
}
