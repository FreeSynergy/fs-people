//! The [`PeopleFeed`] contract: live changes of people.
//!
//! Pattern: Observer — a [`PersonListener`] subscribes and is told about every change of a card;
//! dropping the [`Subscription`] ends it (as in `common.capnp`). [`Registry`] is the reusable subject.
//! A change names a card only: role assignments and same-person links are no part of a feed, so a
//! subscriber does not learn of them (ADR 0029).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use fs_interfaces::Unsupported;
use fs_objects::ObjectId;

use crate::error::Result;

/// What happened to a card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonChange {
    /// A card was created (also by an import).
    Created(ObjectId),
    /// A card was changed (also through a relation, a list membership or the AI marking).
    Updated(ObjectId),
    /// A card was deleted.
    Deleted(ObjectId),
}

/// Receives changes. Called on the thread that made the change: keep it short, hand work off.
pub trait PersonListener: Send + Sync {
    /// A card changed.
    fn changed(&self, change: &PersonChange);
}

/// `person.subscribe`: live updates of cards. Interface status: draft (ADR 0045).
pub trait PeopleFeed: Send + Sync {
    /// Tell `listener` about every change until the returned [`Subscription`] is dropped.
    fn subscribe(&self, _listener: Arc<dyn PersonListener>) -> Result<Subscription> {
        Err(Unsupported::new("person.subscribe", "this provider does not offer it").into())
    }
}

#[derive(Default)]
struct Listeners {
    next: u64,
    entries: Vec<(u64, Arc<dyn PersonListener>)>,
}

/// The subject of the Observer: holds the listeners and notifies them.
#[derive(Default)]
pub struct Registry(Mutex<Listeners>);

impl Registry {
    /// Add a listener; the subscription removes it again.
    pub fn add(self: &Arc<Self>, listener: Arc<dyn PersonListener>) -> Subscription {
        let mut listeners = self.lock();
        listeners.next += 1;
        let id = listeners.next;
        listeners.entries.push((id, listener));
        Subscription {
            id,
            registry: Arc::clone(self),
        }
    }

    /// Tell every listener.
    pub fn notify(&self, change: &PersonChange) {
        let listeners: Vec<_> = self.lock().entries.iter().map(|(_, l)| l.clone()).collect();
        for l in listeners {
            l.changed(change);
        }
    }

    fn lock(&self) -> MutexGuard<'_, Listeners> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Keeps a subscription alive; dropping it unsubscribes.
#[must_use = "dropping the subscription ends it"]
pub struct Subscription {
    id: u64,
    registry: Arc<Registry>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let id = self.id;
        self.registry.lock().entries.retain(|(i, _)| *i != id);
    }
}
