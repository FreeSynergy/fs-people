//! Contact repository over [`fs_store::Store`].

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use fs_store::{Blob, Envelope, ObjectId, Order, Query, Store, StoredObject};
use serde_json::{Value, json};

use crate::card::Card;
use crate::error::{Error, Result};
use crate::index::{fold, index_fields, normalise_email, normalise_phone};
use crate::now_utc;
use crate::validate::validate;
use crate::vcard;

/// AS2 type of a contact envelope (ADR 0020).
pub const PROFILE_TYPE: &str = "Profile";
/// AS2 type of a confirmed same-person link.
pub const RELATIONSHIP_TYPE: &str = "Relationship";
/// `context` of all objects of this program.
pub const CONTEXT: &str = "contacts";
/// Media type of the card content.
pub const CONTENT_TYPE: &str = "application/jscontact+json";
const AS2_TYPE: &str = "application/activity+json";
const SAME_AS: &str = "owl:sameAs";

/// Filter for [`ContactRepo::list`].
#[derive(Clone, Debug, Default)]
pub struct ListFilter {
    /// Only this JSContact `kind`.
    pub kind: Option<String>,
    pub limit: Option<usize>,
    pub offset: usize,
}

/// Result of an import.
#[derive(Clone, Debug, Default)]
pub struct ImportReport {
    pub created: Vec<ObjectId>,
    /// `uid`s that already existed (not overwritten).
    pub skipped: Vec<String>,
    /// Entries that could not be converted or validated.
    pub failed: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchKind {
    Email,
    Phone,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MatchReason {
    pub kind: MatchKind,
    /// The normalised value both cards share.
    pub value: String,
}

/// A suggested `owl:sameAs` link between two cards. Nothing is linked until it is confirmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameAsProposal {
    /// `a < b` (ids are time-ordered), so a pair appears once.
    pub a: ObjectId,
    pub b: ObjectId,
    pub reasons: Vec<MatchReason>,
}

/// Contacts over an object store.
#[derive(Clone)]
pub struct ContactRepo {
    store: Arc<dyn Store>,
    actor: String,
    instance: String,
}

impl ContactRepo {
    /// `actor` is the IRI of the acting person/program (written to the log).
    pub fn new(store: Arc<dyn Store>, actor: impl Into<String>) -> Self {
        Self { store, actor: actor.into(), instance: "local.invalid".into() }
    }

    /// Instance domain for external IRIs `https://<instance>/contacts/<id>` (ADR 0027).
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = instance.into();
        self
    }

    pub fn store(&self) -> &Arc<dyn Store> {
        &self.store
    }

    fn iri(&self, id: ObjectId) -> String {
        format!("https://{}/contacts/{}", self.instance, id)
    }

    fn build_object(&self, card: &Card, existing: Option<&Envelope>, original: Option<Blob>) -> Result<StoredObject> {
        let mut env = match existing {
            Some(e) => e.clone(),
            None => {
                let mut e = Envelope::new(PROFILE_TYPE, card.display_name());
                // A UUIDv7 uid doubles as object id, so a re-import maps onto the same object.
                if let Ok(id) = card.uid.parse::<ObjectId>() {
                    e.id = id;
                }
                e.iri = Some(self.iri(e.id));
                e.context = Some(CONTEXT.into());
                e.attributed_to = Some(self.actor.clone());
                e.to = vec![self.actor.clone()];
                e
            }
        };
        env.name = card.display_name(); // ADR 0032: derived, never typed separately
        env.sensitive = card.sensitive_categories(); // ADR 0029
        let mut o = StoredObject::new(env, CONTENT_TYPE, card.to_value());
        o.index = index_fields(card);
        o.original = original;
        Ok(o)
    }

    // ---- CRUD -------------------------------------------------------------------------------

    pub async fn create(&self, card: Card) -> Result<StoredObject> {
        self.create_inner(card, None).await
    }

    async fn create_inner(&self, mut card: Card, original: Option<Blob>) -> Result<StoredObject> {
        let now = now_utc();
        card.created.get_or_insert_with(|| now.clone());
        card.updated.get_or_insert(now);
        validate(&card)?;
        if self.get_by_uid(&card.uid).await?.is_some() {
            return Err(Error::Conflict(format!("a contact with uid {} exists", card.uid)));
        }
        let obj = self.build_object(&card, None, original)?;
        let id = obj.envelope.id;
        self.store.put(obj, &self.actor).await?;
        self.get(id).await?.ok_or(Error::NotFound(id))
    }

    pub async fn get(&self, id: ObjectId) -> Result<Option<StoredObject>> {
        Ok(self.store.get(id).await?.filter(|o| o.envelope.object_type == PROFILE_TYPE))
    }

    pub async fn get_card(&self, id: ObjectId) -> Result<Option<Card>> {
        self.get(id).await?.map(|o| card_of(&o)).transpose()
    }

    pub async fn get_by_uid(&self, uid: &str) -> Result<Option<StoredObject>> {
        let q = Query::new().of_type(PROFILE_TYPE).with_index("uid", uid).limit(1);
        Ok(self.store.query(&q).await?.into_iter().next())
    }

    /// Replace the card of `id`. The `uid` cannot change; `updated` is set to now.
    pub async fn update(&self, id: ObjectId, mut card: Card) -> Result<StoredObject> {
        let old = self.get(id).await?.ok_or(Error::NotFound(id))?;
        let old_card = card_of(&old)?;
        if old_card.uid != card.uid {
            return Err(Error::Conflict("the uid of a card cannot change".into()));
        }
        card.created = card.created.or(old_card.created);
        card.updated = Some(now_utc());
        validate(&card)?;
        let obj = self.build_object(&card, Some(&old.envelope), None)?;
        self.store.put(obj, &self.actor).await?;
        self.get(id).await?.ok_or(Error::NotFound(id))
    }

    /// Delete a card (undoable through the store). Same-person links that mention it are removed too.
    pub async fn delete(&self, id: ObjectId) -> Result<()> {
        self.get(id).await?.ok_or(Error::NotFound(id))?;
        for rel in self.relationships_of(id).await? {
            self.store.delete(rel.envelope.id, &self.actor).await?;
        }
        self.store.delete(id, &self.actor).await?;
        Ok(())
    }

    // ---- list / search ---------------------------------------------------------------------

    pub async fn list(&self, filter: &ListFilter) -> Result<Vec<StoredObject>> {
        let mut q = Query::new().of_type(PROFILE_TYPE).in_context(CONTEXT).ordered(Order::NameAsc);
        if let Some(k) = &filter.kind {
            q = q.with_index("kind", k.clone());
        }
        // The store orders names bytewise ("NLnet" < "Nextcloud"); sort case-insensitively by the
        // extracted `name_sort` here and page afterwards (DESIGN.md: wanted in fs-store).
        let mut all = self.store.query(&q).await?;
        let key = |o: &StoredObject| o.index.get("name_sort").and_then(|v| v.first().cloned()).unwrap_or_else(|| fold(&o.envelope.name));
        all.sort_by_cached_key(|o| (key(o), o.envelope.id));
        let it = all.into_iter().skip(filter.offset);
        Ok(match filter.limit {
            Some(n) => it.take(n).collect(),
            None => it.collect(),
        })
    }

    /// Exact lookup by (normalised) e-mail address.
    pub async fn find_by_email(&self, email: &str) -> Result<Vec<StoredObject>> {
        let q = Query::new().of_type(PROFILE_TYPE).with_index("email", normalise_email(email));
        Ok(self.store.query(&q).await?)
    }

    /// Exact lookup by (normalised) phone number.
    pub async fn find_by_phone(&self, phone: &str) -> Result<Vec<StoredObject>> {
        let n = normalise_phone(phone);
        if n.is_empty() {
            return Ok(vec![]);
        }
        let q = Query::new().of_type(PROFILE_TYPE).with_index("phone", n);
        Ok(self.store.query(&q).await?)
    }

    /// Case-insensitive substring search over name, nicknames, e-mail, phone (digits) and
    /// organisation. The store has no full-text search yet, so this scans the Profile list
    /// (fine for address books; see DESIGN.md).
    pub async fn search(&self, term: &str) -> Result<Vec<StoredObject>> {
        let t = fold(term);
        if t.is_empty() {
            return self.list(&ListFilter::default()).await;
        }
        let t_email = normalise_email(term);
        let t_phone = normalise_phone(term);
        let phone_needle = t_phone.trim_start_matches('+').to_string();
        let mut hits = vec![];
        for o in self.list(&ListFilter::default()).await? {
            let card = card_of(&o)?;
            let name_hit = fold(&card.display_name()).contains(&t)
                || o.index.get("name_sort").is_some_and(|v| v.iter().any(|s| s.contains(&t)))
                || card.nicknames.values().any(|n| fold(&n.name).contains(&t));
            let email_hit = card.emails.values().any(|e| normalise_email(&e.address).contains(&t_email));
            let min_digits = if t_phone.starts_with('+') { 2 } else { 3 };
            let phone_hit = phone_needle.len() >= min_digits
                && card.phones.values().any(|p| normalise_phone(&p.number).trim_start_matches('+').contains(&phone_needle));
            let org_hit = card.organizations.values().any(|org| org.name.as_deref().is_some_and(|n| fold(n).contains(&t)));
            if name_hit || email_hit || phone_hit || org_hit {
                hits.push(o);
            }
        }
        Ok(hits)
    }

    // ---- import / export -------------------------------------------------------------------

    /// Import a `.vcf` text (one or many vCards). Each block is kept as the original (ADR 0030).
    /// Cards whose `uid` already exists are skipped, never overwritten.
    pub async fn import_vcard(&self, text: &str) -> Result<ImportReport> {
        let mut report = ImportReport::default();
        for r in vcard::import_vcards(text) {
            match r {
                Ok(imp) => {
                    if self.get_by_uid(&imp.card.uid).await?.is_some() {
                        report.skipped.push(imp.card.uid);
                        continue;
                    }
                    let o = self.create_inner(imp.card, Some(imp.original)).await?;
                    report.created.push(o.envelope.id);
                }
                Err(e) => report.failed.push(e.to_string()),
            }
        }
        Ok(report)
    }

    /// Import JSContact JSON (one card or an array), e.g. the demo data.
    pub async fn import_jscontact(&self, json_text: &str) -> Result<ImportReport> {
        let v: Value = serde_json::from_str(json_text)?;
        let items = match v {
            Value::Array(a) => a,
            other => vec![other],
        };
        let mut report = ImportReport::default();
        for item in items {
            match serde_json::from_value::<Card>(item) {
                Ok(card) => {
                    if self.get_by_uid(&card.uid).await?.is_some() {
                        report.skipped.push(card.uid);
                        continue;
                    }
                    let uid = card.uid.clone();
                    match self.create_inner(card, None).await {
                        Ok(o) => report.created.push(o.envelope.id),
                        Err(e @ Error::Invalid(_)) => report.failed.push(format!("{uid}: {e}")),
                        Err(e) => return Err(e),
                    }
                }
                Err(e) => report.failed.push(e.to_string()),
            }
        }
        Ok(report)
    }

    /// The card as vCard 4.0 text.
    pub async fn export_vcard(&self, id: ObjectId) -> Result<String> {
        let card = self.get_card(id).await?.ok_or(Error::NotFound(id))?;
        vcard::card_to_vcard(&card)
    }

    // ---- groups / folders (ADR 0037) -------------------------------------------------------

    /// A group (folder, mailing list): a card with `kind: group` and `members`.
    pub async fn create_group(&self, name: &str) -> Result<StoredObject> {
        self.create(Card::group(name)).await
    }

    pub async fn add_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        let mut g = self.get_card(group).await?.ok_or(Error::NotFound(group))?;
        if !g.is_group() {
            return Err(Error::Conflict(format!("{group} is not a group")));
        }
        if group == member {
            return Err(Error::Conflict("a group cannot contain itself".into()));
        }
        let m = self.get_card(member).await?.ok_or(Error::NotFound(member))?;
        g.members.insert(m.uid, true);
        self.update(group, g).await
    }

    pub async fn remove_member(&self, group: ObjectId, member: ObjectId) -> Result<StoredObject> {
        let mut g = self.get_card(group).await?.ok_or(Error::NotFound(group))?;
        let m = self.get_card(member).await?.ok_or(Error::NotFound(member))?;
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
        let m = self.get_card(member).await?.ok_or(Error::NotFound(member))?;
        let q = Query::new().of_type(PROFILE_TYPE).with_index("kind", "group").with_index("member", m.uid);
        Ok(self.store.query(&q).await?)
    }

    // ---- same-person proposals (ADR 0019, 0029) --------------------------------------------

    /// Candidate pairs that share a normalised e-mail address or phone number.
    ///
    /// Never proposed: cards with sensitive categories (ADR 0029), groups, pairs of different
    /// `kind`, pairs that are already linked. Pure function of the store's state; nothing is written.
    pub async fn find_same_person_proposals(&self) -> Result<Vec<SameAsProposal>> {
        let linked = self.linked_pairs().await?;
        let mut by_value: BTreeMap<(MatchKind, String), Vec<(ObjectId, String)>> = BTreeMap::new();
        for o in self.list(&ListFilter::default()).await? {
            let card = card_of(&o)?;
            if card.is_group() || card.is_sensitive() || !o.envelope.sensitive.is_empty() {
                continue;
            }
            let kind = card.kind().to_string();
            for e in o.index.get("email").into_iter().flatten() {
                by_value.entry((MatchKind::Email, e.clone())).or_default().push((o.envelope.id, kind.clone()));
            }
            for p in o.index.get("phone").into_iter().flatten() {
                // Too-short numbers (extensions, service codes) are not identifying.
                if p.trim_start_matches('+').len() >= 6 {
                    by_value.entry((MatchKind::Phone, p.clone())).or_default().push((o.envelope.id, kind.clone()));
                }
            }
        }
        let mut pairs: BTreeMap<(ObjectId, ObjectId), BTreeSet<MatchReason>> = BTreeMap::new();
        for ((mk, value), ids) in by_value {
            for (i, (a, ka)) in ids.iter().enumerate() {
                for (b, kb) in &ids[i + 1..] {
                    if a == b || ka != kb {
                        continue;
                    }
                    let key = if a < b { (*a, *b) } else { (*b, *a) };
                    if linked.contains(&key) {
                        continue;
                    }
                    pairs.entry(key).or_default().insert(MatchReason { kind: mk, value: value.clone() });
                }
            }
        }
        Ok(pairs
            .into_iter()
            .map(|((a, b), r)| SameAsProposal { a, b, reasons: r.into_iter().collect() })
            .collect())
    }

    /// A person confirmed a proposal: store an AS2 `Relationship` (`owl:sameAs`) between the two
    /// cards. Both cards stay (link, don't merge). Refused for sensitive cards, even if someone
    /// constructs the proposal by hand.
    pub async fn confirm_same_person(&self, a: ObjectId, b: ObjectId) -> Result<StoredObject> {
        if a == b {
            return Err(Error::Conflict("cannot link a card with itself".into()));
        }
        let (oa, ob) = (self.get(a).await?.ok_or(Error::NotFound(a))?, self.get(b).await?.ok_or(Error::NotFound(b))?);
        for o in [&oa, &ob] {
            if !o.envelope.sensitive.is_empty() || card_of(o)?.is_sensitive() {
                return Err(Error::Sensitive(o.envelope.id));
            }
        }
        let key = if a < b { (a, b) } else { (b, a) };
        if self.linked_pairs().await?.contains(&key) {
            return Err(Error::Conflict("these cards are already linked".into()));
        }
        let (iri_a, iri_b) = (oa.envelope.iri.clone().unwrap_or_else(|| self.iri(a)), ob.envelope.iri.clone().unwrap_or_else(|| self.iri(b)));
        let mut env = Envelope::new(RELATIONSHIP_TYPE, format!("{} = {} (same person)", oa.envelope.name, ob.envelope.name));
        env.context = Some(CONTEXT.into());
        env.attributed_to = Some(self.actor.clone());
        env.to = vec![self.actor.clone()];
        env.iri = Some(format!("https://{}/contacts/{}", self.instance, env.id));
        let content = json!({
            "@context": ["https://www.w3.org/ns/activitystreams", {"owl": "http://www.w3.org/2002/07/owl#"}],
            "type": RELATIONSHIP_TYPE,
            "subject": iri_a,
            "relationship": SAME_AS,
            "object": iri_b,
        });
        let obj = StoredObject::new(env, AS2_TYPE, content)
            .with_index("relationship", SAME_AS)
            .with_index("subject", key.0.to_string())
            .with_index("object", key.1.to_string());
        let id = obj.envelope.id;
        self.store.put(obj, &self.actor).await?;
        self.store.get(id).await?.ok_or(Error::NotFound(id))
    }

    /// Remove a same-person link (the store's undo can bring it back).
    pub async fn unlink(&self, relationship: ObjectId) -> Result<()> {
        self.store.delete(relationship, &self.actor).await?;
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

    async fn relationships_of(&self, id: ObjectId) -> Result<Vec<StoredObject>> {
        let mut out = vec![];
        for key in ["subject", "object"] {
            let q = Query::new().of_type(RELATIONSHIP_TYPE).in_context(CONTEXT).with_index(key, id.to_string());
            out.extend(self.store.query(&q).await?);
        }
        Ok(out)
    }

    async fn linked_pairs(&self) -> Result<BTreeSet<(ObjectId, ObjectId)>> {
        let q = Query::new().of_type(RELATIONSHIP_TYPE).in_context(CONTEXT).with_index("relationship", SAME_AS);
        Ok(self
            .store
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
    let get = |k: &str| r.index.get(k).and_then(|v| v.first()).and_then(|s| s.parse().ok());
    (get("subject"), get("object"))
}

/// Parse the content of a stored Profile.
pub fn card_of(o: &StoredObject) -> Result<Card> {
    Ok(serde_json::from_value(o.content.clone())?)
}
