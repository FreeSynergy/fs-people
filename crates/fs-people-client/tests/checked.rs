//! `update_checked` through the client: compare-and-set against the real server and the real store.
//!
//! Integration tests against the trait; no stand-ins.

mod common;

use common::{Served, add, person};
use fs_people_client::RevisedPeople;
use fs_people_core::{Card, Error, People, Revision, card_of};
use fs_objects::ObjectId;

/// The revision the server gives (the client cannot derive it: the wire keeps time in milliseconds).
async fn read(people: &dyn RevisedPeople, id: ObjectId) -> (Revision, Card) {
    let read = people.get_revised(id).await.unwrap().unwrap();
    (read.revision, card_of(&read.object).unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_revision_read_writes_and_a_stale_one_writes_nothing() {
    let served = Served::start().await;
    let people = served.client();
    let id = add(&people, person("Ada", "ada@x.org")).await;
    let (revision, mut card) = read(&people, id).await;

    card.add_phone("+49 30 1");
    let done = people.update_checked(id, revision, card).await.unwrap();
    assert_eq!(done.envelope.id, id);
    assert_ne!(
        Revision::of(&done),
        revision,
        "a write changes the revision"
    );

    let (_, mut other) = read(&people, id).await;
    other.add_phone("+49 30 2");
    let refused = people.update_checked(id, revision, other).await;
    assert!(
        matches!(refused, Err(Error::Changed(got)) if got == id),
        "{refused:?}"
    );
    let (_, now) = read(&people, id).await;
    assert_eq!(now.phones.len(), 1, "nothing was written");
}

#[tokio::test(flavor = "multi_thread")]
async fn two_writers_with_the_same_revision_have_one_winner() {
    let served = Served::start().await;
    let (one, two) = (served.client(), served.client());
    let id = add(&one, person("Ada", "ada@x.org")).await;
    let (revision, card) = read(&one, id).await;
    let (mut a, mut b) = (card.clone(), card);
    a.add_phone("+49 30 1");
    b.add_phone("+49 30 2");
    let (ra, rb) = tokio::join!(
        one.update_checked(id, revision, a),
        two.update_checked(id, revision, b)
    );
    assert_eq!(ra.is_ok() as u8 + rb.is_ok() as u8, 1, "{ra:?} {rb:?}");
    let loser = if ra.is_ok() { rb } else { ra };
    assert!(matches!(loser, Err(Error::Changed(_))), "{loser:?}");
    let (_, now) = read(&one, id).await;
    assert_eq!(now.phones.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_person_is_not_found_and_the_client_offers_it() {
    let served = Served::start().await;
    let people = served.client();
    let ghost = ObjectId::new();
    let got = people
        .update_checked(ghost, Revision::from(1), person("X", "x@x.org"))
        .await;
    assert!(matches!(got, Err(Error::NotFound(_))), "{got:?}");
    assert!(people.capabilities().is_supported("person.update"));
    assert!(people.get_revised(ghost).await.unwrap().is_none());
}
