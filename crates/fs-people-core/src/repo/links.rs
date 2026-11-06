//! Confirmed same-person links (AS2 `Relationship`, `owl:sameAs`).
//!
//! Pattern: Repository — links are ordinary store objects with `subject` / `object` index
//! fields, so they are logged, undoable and queryable like cards. Link, don't merge: both cards stay.

use std::collections::BTreeSet;

use fs_store::{Cond, Envelope, ObjectId, Op, Query, StoredObject};
use serde_json::json;

use super::consts::{AS2_TYPE, CONTEXT, RELATIONSHIP_TYPE, SAME_AS};
use super::envelope::card_of;
use super::handle::PeopleRepo;
use crate::error::{Error, Result};

impl PeopleRepo {
    /// A person confirmed a proposal: store an AS2 `Relationship` (`owl:sameAs`) between the two
    /// cards. Both cards stay (link, don't merge). Refused for sensitive cards, even if someone
    /// constructs the proposal by hand.
    pub async fn confirm_same_person(&self, a: ObjectId, b: ObjectId) -> Result<StoredObject> {
        if a == b {
            return Err(Error::Conflict("cannot link a card with itself".into()));
        }
        let oa = self.get(a).await?.ok_or(Error::NotFound(a))?;
        let ob = self.get(b).await?.ok_or(Error::NotFound(b))?;
        for o in [&oa, &ob] {
            if !o.envelope.sensitive.is_empty() || card_of(o)?.is_sensitive() {
                return Err(Error::Sensitive(o.envelope.id));
            }
        }
        let key = if a < b { (a, b) } else { (b, a) };
        if self.linked_pairs().await?.contains(&key) {
            return Err(Error::Conflict("these cards are already linked".into()));
        }
        let iri_of = |o: &StoredObject, id| o.envelope.iri.clone().unwrap_or_else(|| self.iri(id));
        let name = format!("{} = {} (same person)", oa.envelope.name, ob.envelope.name);
        let mut env = Envelope::new(RELATIONSHIP_TYPE, name);
        env.context = Some(CONTEXT.into());
        env.attributed_to = Some(self.actor.clone());
        env.to = vec![self.actor.clone()];
        env.iri = Some(self.iri(env.id));
        let content = json!({
            "@context": ["https://www.w3.org/ns/activitystreams", {"owl": "http://www.w3.org/2002/07/owl#"}],
            "type": RELATIONSHIP_TYPE,
            "subject": iri_of(&oa, a),
            "relationship": SAME_AS,
            "object": iri_of(&ob, b),
        });
        let obj = StoredObject::new(env, AS2_TYPE, content)
            .with_index("relationship", SAME_AS)
            .with_index("subject", key.0.to_string())
            .with_index("object", key.1.to_string());
        let id = obj.envelope.id;
        self.objects.put(obj, &self.actor).await?;
        self.objects.get(id).await?.ok_or(Error::NotFound(id))
    }

    /// Remove a same-person link (the store's undo can bring it back). Any other object with this
    /// id stays.
    pub async fn unlink(&self, relationship: ObjectId) -> Result<()> {
        let found = self.objects.get(relationship).await?;
        let is_link = found.as_ref().is_some_and(|o| {
            o.envelope.object_type == RELATIONSHIP_TYPE
                && o.index
                    .get("relationship")
                    .is_some_and(|v| v.iter().any(|r| r == SAME_AS))
        });
        if !is_link {
            return Err(fs_store::Error::NotFound(relationship).into());
        }
        self.objects.delete(relationship, &self.actor).await?;
        Ok(())
    }

    /// Cards linked to `id` by `owl:sameAs`.
    pub async fn linked_to(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        let mut out = vec![];
        for r in self.relationships_of(id).await? {
            let (s, o) = rel_ids(&r);
            out.extend([s, o].into_iter().flatten().filter(|x| *x != id));
        }
        Ok(out)
    }

    pub(super) async fn relationships_of(&self, id: ObjectId) -> Result<Vec<StoredObject>> {
        let is = |key: &str| Cond::Index {
            key: key.into(),
            op: Op::Eq,
            value: id.to_string(),
        };
        let q = Query::new()
            .of_type(RELATIONSHIP_TYPE)
            .in_context(CONTEXT)
            .or(is("subject"))
            .or(is("object"));
        Ok(self.reads.query(&q).await?)
    }

    pub(super) async fn linked_pairs(&self) -> Result<BTreeSet<(ObjectId, ObjectId)>> {
        let q = Query::new()
            .of_type(RELATIONSHIP_TYPE)
            .in_context(CONTEXT)
            .with_index("relationship", SAME_AS);
        Ok(self
            .reads
            .query(&q)
            .await?
            .iter()
            .filter_map(|r| match rel_ids(r) {
                (Some(a), Some(b)) => Some((a, b)),
                _ => None,
            })
            .collect())
    }
}

fn rel_ids(r: &StoredObject) -> (Option<ObjectId>, Option<ObjectId>) {
    let get = |k: &str| {
        r.index
            .get(k)
            .and_then(|v| v.first())
            .and_then(|s| s.parse().ok())
    };
    (get("subject"), get("object"))
}
