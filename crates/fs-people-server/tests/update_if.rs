//! `Persons.updateIf` over a real socket: compare-and-set on the revision every answer carries.

mod common;

use common::{
    CONFLICT, NOT_FOUND, card, create, expect_problem, id_of, json_of, name_of, persons_of,
    revision_now, revision_of, update_if, with_server,
};
use fs_interfaces::rpc::write_uuid;

fn marked(uid: &str, name: &str) -> String {
    format!(
        r#"{{"@type":"Card","version":"1.0","uid":"{uid}","name":{{"full":"{name}"}},"freesynergy.net:sensitive":["pd:HealthRecord"]}}"#
    )
}

#[tokio::test(flavor = "current_thread")]
async fn every_answer_carries_a_revision_and_each_write_changes_it() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let mut make = persons.create_request();
        make.get().set_jscontact(card("u1", "Ada", "a@x.org").as_str());
        let made = make.send().promise.await.unwrap();
        let person = made.get().unwrap().get_person().unwrap();
        let id = id_of(person);
        let first = revision_of(person);

        assert_eq!(revision_now(&persons, id).await, first);
        let page = persons.list_request().send().promise.await.unwrap();
        let listed = page.get().unwrap().get_page().unwrap().get_items().unwrap();
        assert_eq!(revision_of(listed.get(0)), first);

        let mut update = persons.update_request();
        write_uuid(update.get().init_id(), Some(id));
        update.get().set_jscontact(card("u1", "Ada K", "a@x.org").as_str());
        let done = update.send().promise.await.unwrap();
        let second = revision_of(done.get().unwrap().get_person().unwrap());
        assert_ne!(second, first);
        assert_eq!(revision_now(&persons, id).await, second);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn update_if_writes_on_the_revision_read_and_refuses_a_stale_one() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, "u1", "Ada", "a@x.org").await;
        let read = revision_now(&persons, id).await;

        let done = update_if(&persons, id, read, &card("u1", "Ada B", "a@x.org"))
            .await
            .unwrap();
        let person = done.get().unwrap().get_person().unwrap();
        assert_eq!(name_of(person), "Ada B");
        let now = revision_of(person);
        assert_ne!(now, read);

        let detail = expect_problem(
            update_if(&persons, id, read, &card("u1", "Ada C", "a@x.org")).await,
            CONFLICT,
        );
        assert!(detail.starts_with("conflict:"), "{detail}");
        assert!(!detail.contains(&format!("{now:016x}")), "{detail}");
        assert_eq!(revision_now(&persons, id).await, now, "nothing was written");
        let mut get = persons.get_request();
        write_uuid(get.get().init_id(), Some(id));
        let got = get.send().promise.await.unwrap();
        let card_now = json_of(got.get().unwrap().get_person().unwrap());
        assert_eq!(card_now["name"]["full"], "Ada B");

        let ghost = common::unknown_id();
        expect_problem(
            update_if(&persons, ghost, 1, &card("u9", "X", "x@x.org")).await,
            NOT_FOUND,
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn of_two_writers_on_one_revision_exactly_one_wins() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, "u1", "Ada", "a@x.org").await;
        for round in 0..10 {
            let read = revision_now(&persons, id).await;
            let (card_a, card_b) = (
                card("u1", &format!("A{round}"), "a@x.org"),
                card("u1", &format!("B{round}"), "a@x.org"),
            );
            let a = update_if(&persons, id, read, &card_a);
            let b = update_if(&persons, id, read, &card_b);
            let (a, b) = tokio::join!(a, b);
            let (won, lost) = match (a.is_ok(), b.is_ok()) {
                (true, false) => (format!("A{round}"), b),
                (false, true) => (format!("B{round}"), a),
                other => panic!("round {round}: {other:?}"),
            };
            expect_problem(lost, CONFLICT);
            let mut get = persons.get_request();
            write_uuid(get.get().init_id(), Some(id));
            let got = get.send().promise.await.unwrap();
            assert_eq!(name_of(got.get().unwrap().get_person().unwrap()), won);
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_sensitive_card_has_a_revision_and_a_refusal_says_no_more() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let mut make = persons.create_request();
        make.get().set_jscontact(marked("s1", "Sam").as_str());
        let made = make.send().promise.await.unwrap();
        let person = made.get().unwrap().get_person().unwrap();
        let id = id_of(person);
        let read = revision_of(person);

        let done = update_if(&persons, id, read, &marked("s1", "Sam B")).await.unwrap();
        let now = revision_of(done.get().unwrap().get_person().unwrap());
        assert_ne!(now, read);
        let detail = expect_problem(
            update_if(&persons, id, read, &marked("s1", "Sam C")).await,
            CONFLICT,
        );
        assert!(detail.starts_with("conflict:"), "{detail}");
        assert!(
            !detail.contains("Sam") && !detail.contains("pd:") && !detail.contains(&format!("{now:016x}")),
            "{detail}"
        );
    })
    .await;
}
