//! Cards stored before the index `uid_key` existed are found over the wire, in any spelling, and known to
//! `create` and an import.

mod common;

use common::{CONFLICT, card, expect_problem, id_of, persons_of, with_store};
use fs_people_core::Card;
use fs_store::{Envelope, StoredObject};

const UUID: &str = "9f8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d";

#[tokio::test(flavor = "current_thread")]
async fn an_older_card_is_found_and_not_doubled() {
    with_store(|root, store| async move {
        let persons = persons_of(&root).await;
        let uid = format!("urn:uuid:{UUID}");
        let mut older = Card::individual("Older");
        older.uid.clone_from(&uid);
        let mut envelope = Envelope::new("Profile", "Older");
        envelope.context = Some("people".into());
        let id = envelope.id.as_uuid();
        let object = StoredObject::new(envelope, "application/jscontact+json", older.to_value())
            .with_index("uid", uid.as_str())
            .with_index("kind", "individual")
            .with_index("name_sort", "older");
        store.put(object, common::ME).await.unwrap();

        for spelling in [
            UUID.to_string(),
            uid.clone(),
            UUID.to_uppercase(),
            UUID.replace('-', ""),
        ] {
            let mut get = persons.get_by_uid_request();
            get.get().set_uid(spelling.as_str());
            let answer = get.send().promise.await.unwrap();
            assert!(answer.get().unwrap().get_found(), "{spelling}");
            assert_eq!(
                id_of(answer.get().unwrap().get_person().unwrap()),
                id,
                "{spelling}"
            );
        }

        let mut again = persons.create_request();
        again
            .get()
            .set_jscontact(card(UUID, "Twin", "t@x.org").as_str());
        expect_problem(again.send().promise.await, CONFLICT);
    })
    .await;
}
