//! `People::update_checked`, tested against the trait: the card is replaced only if it still has the
//! revision the caller read; of two writers on one revision exactly one wins; a wrong revision writes
//! nothing.

mod common;

use common::person;
use common::yielding::yielding_repo;
use fs_people_core::{Card, Error, People, Revision, card_of};

fn renamed(of: &Card, name: &str) -> Card {
    let mut card = of.clone();
    card.name = Card::individual(name).name;
    card
}

#[tokio::test(flavor = "current_thread")]
async fn the_revision_read_writes_and_the_next_one_differs() {
    let r = common::repo().await;
    let first = People::create(&r, person("Ada", "ada@x.org"))
        .await
        .unwrap();
    let card = card_of(&first).unwrap();
    let seen = Revision::of(&first);
    assert_eq!(Revision::of(&first), seen, "reading again changes nothing");

    let second = r
        .update_checked(first.envelope.id, seen, renamed(&card, "Ada King"))
        .await
        .unwrap();
    assert_eq!(second.envelope.name, "Ada King");
    assert_ne!(
        Revision::of(&second),
        seen,
        "every write changes the revision"
    );
    let read = People::get(&r, first.envelope.id).await.unwrap().unwrap();
    assert_eq!(Revision::of(&read), Revision::of(&second));
}

#[tokio::test(flavor = "current_thread")]
async fn a_wrong_revision_writes_nothing_and_does_not_name_the_right_one() {
    let (r, store) = common::setup().await;
    let first = People::create(&r, person("Ada", "ada@x.org"))
        .await
        .unwrap();
    let card = card_of(&first).unwrap();
    let stale = Revision::of(&first);
    let current = People::update(&r, first.envelope.id, renamed(&card, "Ada B"))
        .await
        .unwrap();
    let log_before = store.count(&fs_objects::Query::new()).await.unwrap();

    let err = r
        .update_checked(first.envelope.id, stale, renamed(&card, "Ada C"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::Changed(id) if id == first.envelope.id),
        "{err}"
    );
    let text = err.to_string();
    assert!(text.starts_with("conflict:"), "{text}");
    assert!(
        !text.contains(&Revision::of(&current).to_string()),
        "{text}"
    );

    let now = People::get(&r, first.envelope.id).await.unwrap().unwrap();
    assert_eq!(now.content, current.content, "nothing was written");
    assert_eq!(Revision::of(&now), Revision::of(&current));
    assert_eq!(
        store.count(&fs_objects::Query::new()).await.unwrap(),
        log_before
    );
}

#[tokio::test(flavor = "current_thread")]
async fn an_unknown_person_is_not_found() {
    let r = common::repo().await;
    let ghost = fs_objects::Envelope::new("x", "x").id;
    let err = r
        .update_checked(ghost, Revision::from(1), Card::individual("Nobody"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)), "{err}");
}

#[tokio::test(flavor = "current_thread")]
async fn of_two_writers_on_one_revision_exactly_one_wins() {
    for round in 0..20 {
        let r = yielding_repo().await;
        let first = People::create(&*r, person("Ada", "ada@x.org"))
            .await
            .unwrap();
        let id = first.envelope.id;
        let card = card_of(&first).unwrap();
        let seen = Revision::of(&first);

        let mut writers = tokio::task::JoinSet::new();
        for name in ["Writer A", "Writer B"] {
            let (r, card) = (r.clone(), renamed(&card, name));
            writers.spawn(async move { (name, r.update_checked(id, seen, card).await) });
        }
        let (mut won, mut lost) = (vec![], vec![]);
        while let Some(done) = writers.join_next().await {
            match done.unwrap() {
                (name, Ok(_)) => won.push(name),
                (_, Err(Error::Changed(_))) => lost.push(()),
                (_, Err(other)) => panic!("{other}"),
            }
        }
        assert_eq!((won.len(), lost.len()), (1, 1), "round {round}");
        let now = People::get(&*r, id).await.unwrap().unwrap();
        assert_eq!(now.envelope.name, won[0], "the loser wrote nothing");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_sensitive_card_has_a_revision_like_any_other_and_says_no_more_when_refused() {
    let r = common::repo().await;
    let mut marked = person("Sam", "sam@x.org");
    marked.extra.insert(
        "freesynergy.net:sensitive".into(),
        serde_json::json!(["pd:HealthRecord"]),
    );
    let stored = People::create(&r, marked).await.unwrap();
    let card = card_of(&stored).unwrap();
    let seen = Revision::of(&stored);
    let next = r
        .update_checked(stored.envelope.id, seen, renamed(&card, "Sam B"))
        .await
        .unwrap();
    assert_ne!(Revision::of(&next), seen);
    let err = r
        .update_checked(stored.envelope.id, seen, renamed(&card, "Sam C"))
        .await
        .unwrap_err()
        .to_string();
    assert!(!err.contains("Sam") && !err.contains("pd:"), "{err}");
    assert!(!err.contains(&Revision::of(&next).to_string()), "{err}");
}
