//! The revision of a stored card, for compare-and-set.
//!
//! Pattern: Newtype — a number that is only ever compared for equality, so it cannot be taken for a
//! counter, a time or an order. Why this pattern: the contract (`people.capnp`, `Person.revision`)
//! says "compare for equality only"; a type with `==` and no `<` keeps the code to that.
//! The trait is [`People::update_checked`](crate::People::update_checked) (Repository with an
//! optimistic lock): a writer hands back the revision it read, and the card is replaced only if it
//! still has it.
//!
//! The revision changes at every write that changes the state; an undo in the store may bring an
//! earlier one back; the same state always has the same revision, so no write is lost by that.
//!
//! The revision is not stored: it is derived from what is stored, so there is nothing to migrate and no
//! second place that can disagree. It is the store's `updated` (microseconds) and the card as JSON
//! (FNV-1a), so two writes within one tick still differ, and a card written to the same content differs
//! by its time. ADR 0029: it is computed from exactly what an answer holds (the stored card and its
//! time), the same way for every card, sensitive or not; it carries no flag, no length and no part of
//! the card that the answer does not show, and a refusal does not name the revision found.

use std::fmt;

use fs_store::StoredObject;

/// What a card was when it was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Revision(u64);

impl Revision {
    /// The revision of the object as stored now.
    pub fn of(object: &StoredObject) -> Self {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let micros = object.envelope.updated.timestamp_micros();
        let step = |hash: u64, byte: u8| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        for byte in micros.to_le_bytes() {
            hash = step(hash, byte);
        }
        for byte in object.content.to_string().bytes() {
            hash = step(hash, byte);
        }
        Self(hash)
    }

    /// The revision as it travels over a process boundary (`Person.revision`).
    pub fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for Revision {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}
