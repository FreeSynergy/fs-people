//! Changes of the provider to a subscribed consumer.
//!
//! Pattern: Observer + Adapter — the core calls its listener on the thread that made the change; this
//! one only puts the change into a channel. The shared forwarder adds the envelope and sends it.

use std::sync::Arc;

use fs_interfaces::common_capnp::{change_listener, subscription};
use fs_interfaces::rpc::{ChangeSender, Notice, change_channel, forward};
use fs_people_core::{People, PeopleFeed, PersonChange, PersonListener};

use crate::errors::rpc_error;

struct ToChannel(ChangeSender<PersonChange>);

impl PersonListener for ToChannel {
    fn changed(&self, change: &PersonChange) {
        self.0.send(*change);
    }
}

/// Subscribe `listener` until the returned capability is dropped.
pub(crate) fn subscribe(
    feed: &Arc<dyn PeopleFeed>,
    people: Arc<dyn People>,
    listener: change_listener::Client,
) -> capnp::Result<subscription::Client> {
    let (tx, rx) = change_channel();
    let core_subscription = feed.subscribe(Arc::new(ToChannel(tx))).map_err(rpc_error)?;
    Ok(forward(rx, core_subscription, listener, move |change| {
        let people = people.clone();
        async move { notice(people.as_ref(), change).await }
    }))
}

/// A card that is gone again by now has nothing to report but its own `Deleted`, which follows.
async fn notice(people: &dyn People, change: PersonChange) -> Option<Notice> {
    match change {
        PersonChange::Created(id) => Some(Notice::Created(people.get(id).await.ok()??.envelope)),
        PersonChange::Updated(id) => Some(Notice::Updated(people.get(id).await.ok()??.envelope)),
        PersonChange::Deleted(id) => Some(Notice::Deleted(id)),
    }
}
