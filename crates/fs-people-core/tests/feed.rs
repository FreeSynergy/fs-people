//! `person.subscribe`: what a listener is told, and what it is not.

mod common;

use std::sync::{Arc, Mutex};

use common::{person, repo};
use fs_people_core::{Card, PeopleFeed, PersonChange, PersonListener};
use serde_json::Map;

#[derive(Default)]
struct Seen(Mutex<Vec<PersonChange>>);

impl PersonListener for Seen {
    fn changed(&self, change: &PersonChange) {
        self.0.lock().unwrap().push(*change);
    }
}

#[tokio::test]
async fn cards_are_told_until_the_subscription_is_dropped() {
    let r = repo().await;
    let seen = Arc::new(Seen::default());
    let subscription = r.subscribe(seen.clone()).unwrap();

    let id = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.relate(id, "urn:other", "friend").await.unwrap();
    r.delete(id).await.unwrap();
    assert_eq!(
        *seen.0.lock().unwrap(),
        [
            PersonChange::Created(id),
            PersonChange::Updated(id),
            PersonChange::Deleted(id)
        ]
    );

    drop(subscription);
    r.create(Card::individual("Bob")).await.unwrap();
    assert_eq!(seen.0.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn roles_and_links_are_no_part_of_the_feed() {
    let r = repo().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let b = r
        .create(person("Ada K", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let seen = Arc::new(Seen::default());
    let _subscription = r.subscribe(seen.clone()).unwrap();

    let held = r
        .assign_role(a, "member", Some("https://x/groups/t"), Map::new())
        .await
        .unwrap();
    r.unassign_role(held.id).await.unwrap();
    let link = r.confirm_same_person(a, b).await.unwrap();
    r.unlink(link.envelope.id).await.unwrap();
    assert!(seen.0.lock().unwrap().is_empty());
}
