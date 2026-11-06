//! What must not leave: roles on a person, `Relationship` objects, and reaching them by id.

mod common;

use common::{
    CLUB, NOT_FOUND, SENSITIVE, create, expect_problem, id_of, json_of, name_of, persons_of,
    roles_of, with_server,
};
use fs_interfaces::rpc::{read_uuid, write_uuid};

#[tokio::test(flavor = "current_thread")]
async fn no_answer_about_a_person_carries_a_role() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let ada = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let mut assign = roles.assign_request();
        write_uuid(assign.get().init_person(), Some(ada));
        assign.get().set_role("member");
        assign.get().set_scope(CLUB);
        assign.send().promise.await.unwrap();

        let mut get = persons.get_request();
        write_uuid(get.get().init_id(), Some(ada));
        let got = get.send().promise.await.unwrap();
        let person = got.get().unwrap().get_person().unwrap();
        assert_eq!(person.get_roles().unwrap().len(), 0);
        let card = json_of(person).to_string();
        assert!(!card.contains("member") && !card.contains(CLUB), "{card}");

        let answer = persons.list_request().send().promise.await.unwrap();
        let page = answer.get().unwrap().get_page().unwrap();
        assert_eq!(
            page.get_items().unwrap().get(0).get_roles().unwrap().len(),
            0
        );

        let mut by_mail = persons.find_by_email_request();
        by_mail.get().set_email("ada@x.org");
        let answer = by_mail.send().promise.await.unwrap();
        let items = answer.get().unwrap().get_items().unwrap();
        assert_eq!(items.get(0).get_roles().unwrap().len(), 0);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_relationship_is_no_person_and_stays_what_it_is() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let a = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let b = create(&persons, "uid-2", "A. King", "ada@x.org").await;

        let mut assign = roles.assign_request();
        write_uuid(assign.get().init_person(), Some(a));
        assign.get().set_role("member");
        assign.get().set_scope(CLUB);
        let answer = assign.send().promise.await.unwrap();
        let held = answer.get().unwrap().get_assignment().unwrap();
        let held = read_uuid(held.get_id().unwrap()).unwrap();

        let mut confirm = persons.confirm_same_request();
        write_uuid(confirm.get().init_a(), Some(a));
        write_uuid(confirm.get().init_b(), Some(b));
        let answer = confirm.send().promise.await.unwrap();
        let link = read_uuid(answer.get().unwrap().get_link().unwrap()).unwrap();

        for relationship in [held, link] {
            let mut get = persons.get_request();
            write_uuid(get.get().init_id(), Some(relationship));
            let answer = get.send().promise.await.unwrap();
            assert!(!answer.get().unwrap().get_found(), "read as a person");

            let mut delete = persons.delete_request();
            write_uuid(delete.get().init_id(), Some(relationship));
            expect_problem(delete.send().promise.await, NOT_FOUND);

            let mut export = persons.export_text_request();
            write_uuid(export.get().init_id(), Some(relationship));
            export.get().set_media_type("text/vcard");
            expect_problem(export.send().promise.await, NOT_FOUND);
        }

        // Each removal reaches only its own kind of object, and never a card.
        for wrong in [held, a] {
            let mut unlink = persons.unlink_same_request();
            write_uuid(unlink.get().init_link(), Some(wrong));
            expect_problem(unlink.send().promise.await, NOT_FOUND);
        }
        for wrong in [link, a] {
            let mut unassign = roles.unassign_request();
            write_uuid(unassign.get().init_assignment(), Some(wrong));
            expect_problem(unassign.send().promise.await, NOT_FOUND);
        }
        let mut get = persons.get_request();
        write_uuid(get.get().init_id(), Some(a));
        let answer = get.send().promise.await.unwrap();
        let person = answer.get().unwrap().get_person().unwrap();
        assert_eq!(name_of(person), "Ada");
        id_of(person);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn sensitive_cards_are_not_proposed_or_linked() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let text = r#"{"@type":"Card","version":"1.0","uid":"uid-1","name":{"full":"Ada"},
            "emails":{"e1":{"address":"ada@x.org"}},"freesynergy.net:sensitive":["pd:HealthRecord"]}"#;
        let mut req = persons.create_request();
        req.get().set_jscontact(text);
        let answer = req.send().promise.await.unwrap();
        let a = id_of(answer.get().unwrap().get_person().unwrap());
        let b = create(&persons, "uid-2", "A. King", "ada@x.org").await;

        let answer = persons.proposals_request().send().promise.await.unwrap();
        assert_eq!(answer.get().unwrap().get_items().unwrap().len(), 0);

        let mut confirm = persons.confirm_same_request();
        write_uuid(confirm.get().init_a(), Some(a));
        write_uuid(confirm.get().init_b(), Some(b));
        expect_problem(confirm.send().promise.await, SENSITIVE);
    })
    .await;
}
