//! Listing, lookup and search.
//!
//! Pattern: Repository (read side) — builds backend-neutral `Query` values; filtering, ordering
//! and paging are done by the store, not in Rust.

use fs_store::{Cond, Op, Query, StoredObject};

use super::consts::{CONTEXT, PROFILE_TYPE};
use super::handle::ContactRepo;
use super::types::ListFilter;
use crate::error::Result;
use crate::index::{normalise_email, normalise_phone};

impl ContactRepo {
    /// Profiles ordered case-insensitively by `name_sort` ("surname, given"); filtering, ordering
    /// and paging are done by the store.
    pub async fn list(&self, filter: &ListFilter) -> Result<Vec<StoredObject>> {
        let mut q = Query::new()
            .of_type(PROFILE_TYPE)
            .in_context(CONTEXT)
            .order_by_index("name_sort");
        if let Some(k) = &filter.kind {
            q = q.with_index("kind", k.clone());
        }
        q.offset = filter.offset;
        q.limit = filter.limit;
        Ok(self.reads.query(&q).await?)
    }

    /// Profiles whose card describes this actor IRI / DID (envelope `describes`).
    pub async fn find_by_describes(&self, iri: &str) -> Result<Vec<StoredObject>> {
        let q = Query::new().of_type(PROFILE_TYPE).and(Cond::Describes {
            op: Op::Eq,
            value: iri.into(),
        });
        Ok(self.reads.query(&q).await?)
    }

    /// Exact lookup by (normalised) e-mail address.
    pub async fn find_by_email(&self, email: &str) -> Result<Vec<StoredObject>> {
        let q = Query::new()
            .of_type(PROFILE_TYPE)
            .with_index("email", normalise_email(email));
        Ok(self.reads.query(&q).await?)
    }

    /// Exact lookup by (normalised) phone number.
    pub async fn find_by_phone(&self, phone: &str) -> Result<Vec<StoredObject>> {
        let n = normalise_phone(phone);
        if n.is_empty() {
            return Ok(vec![]);
        }
        let q = Query::new().of_type(PROFILE_TYPE).with_index("phone", n);
        Ok(self.reads.query(&q).await?)
    }

    /// Case-insensitive (Unicode-normalised) substring search over name, nicknames, e-mail,
    /// phone (digits) and organisation, evaluated by the store (`Op::Contains` over an
    /// OR-group). Result order as in [`ContactRepo::list`].
    pub async fn search(&self, term: &str) -> Result<Vec<StoredObject>> {
        let term = term.trim();
        if term.is_empty() {
            return self.list(&ListFilter::default()).await;
        }
        let has = |key: &str, value: String| Cond::Index {
            key: key.into(),
            op: Op::Contains,
            value,
        };
        let mut q = Query::new()
            .of_type(PROFILE_TYPE)
            .in_context(CONTEXT)
            .order_by_index("name_sort")
            .or(Cond::Name {
                op: Op::Contains,
                value: term.into(),
            })
            .or(has("name_sort", term.into()))
            .or(has("nickname", term.into()))
            .or(has("email", normalise_email(term)))
            .or(has("org", term.into()));
        // Digits only count as a phone search when there are enough of them to mean something.
        let t_phone = normalise_phone(term);
        let phone_needle = t_phone.trim_start_matches('+');
        let min_digits = if t_phone.starts_with('+') { 2 } else { 3 };
        if phone_needle.len() >= min_digits {
            q = q.or(has("phone", phone_needle.to_string()));
        }
        Ok(self.reads.query(&q).await?)
    }
}
