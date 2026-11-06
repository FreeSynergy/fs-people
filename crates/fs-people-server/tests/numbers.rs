//! A number that cannot be kept exactly is refused at the door; a `-0` an older card holds is not new.

mod common;

use common::{BAD_REQUEST, INVALID, ME, expect_problem, json_of, persons_of, with_store};
use fs_interfaces::rpc::write_uuid;
use fs_people_core::Card;
use fs_store::{Envelope, StoredObject};

fn with_number(uid: &str, number: &str) -> String {
    format!(
        r#"{{"@type":"Card","version":"1.0","uid":"{uid}","name":{{"full":"N"}},"example.org:n":{number}}}"#
    )
}

#[tokio::test(flavor = "current_thread")]
async fn a_number_that_would_change_does_not_come_in() {
    with_store(|root, _| async move {
        let persons = persons_of(&root).await;
        for number in [
            "123456789012345678901234567890",
            "1.2345678901234567890",
            "1e400",
        ] {
            let mut inexact = persons.create_request();
            inexact
                .get()
                .set_jscontact(with_number("n", number).as_str());
            expect_problem(inexact.send().promise.await, BAD_REQUEST);
        }
        let mut zero = persons.create_request();
        zero.get().set_jscontact(with_number("n", "-0").as_str());
        let detail = expect_problem(zero.send().promise.await, INVALID);
        assert!(detail.contains("-0"), "{detail}");

        let mut fine = persons.create_request();
        fine.get()
            .set_jscontact(with_number("f", "13.049435775302797").as_str());
        let answer = fine.send().promise.await.unwrap();
        let stored = json_of(answer.get().unwrap().get_person().unwrap());
        assert_eq!(
            stored["example.org:n"],
            serde_json::json!(13.049435775302797)
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_negative_zero_an_older_card_holds_can_be_sent_back() {
    with_store(|root, store| async move {
        let persons = persons_of(&root).await;
        let mut older = Card::individual("Old");
        older.uid = "old".into();
        older
            .extra
            .insert("example.org:n".into(), serde_json::json!(-0.0));
        let mut envelope = Envelope::new("Profile", "Old");
        envelope.context = Some("people".into());
        let id = envelope.id.as_uuid();
        let object = StoredObject::new(envelope, "application/jscontact+json", older.to_value())
            .with_index("uid", "old")
            .with_index("kind", "individual");
        store.put(object, ME).await.unwrap();

        let mut update = persons.update_request();
        write_uuid(update.get().init_id(), Some(id));
        update
            .get()
            .set_jscontact(with_number("old", "-0.0").as_str());
        update.send().promise.await.unwrap();

        let clean = common::create(&persons, "clean", "Clean", "c@x.org").await;
        let mut new_one = persons.update_request();
        write_uuid(new_one.get().init_id(), Some(clean));
        new_one
            .get()
            .set_jscontact(with_number("clean", "-0").as_str());
        expect_problem(new_one.send().promise.await, INVALID);
    })
    .await;
}
