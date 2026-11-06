//! Sizes near the limit of a message: a page is cut, a value that could not come back is refused, and
//! nothing ends the connection.

mod common;

use common::{TOO_LARGE, expect_problem, name_of, persons_of, roles_of, with_server};
use fs_interfaces::rpc::MAX_INTAKE_BYTES;
use fs_people_core::MAX_CARD_BYTES;
use serde_json::json;

const BIG: usize = 12 * 1024 * 1024;

fn big_card(uid: &str, name: &str, note: usize) -> String {
    json!({"@type": "Card", "version": "1.0", "uid": uid, "name": {"full": name},
        "emails": {"e1": {"address": "same@x.org"}},
        "notes": {"n1": {"note": "x".repeat(note)}}})
    .to_string()
}

#[test]
fn the_core_keeps_what_the_contract_can_bring_back() {
    assert_eq!(MAX_CARD_BYTES, MAX_INTAKE_BYTES);
}

#[tokio::test(flavor = "current_thread")]
async fn a_page_is_cut_and_an_answer_without_a_cursor_refuses() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        for (uid, name) in [("big-1", "A"), ("big-2", "B"), ("big-3", "C")] {
            let mut create = persons.create_request();
            create
                .get()
                .set_jscontact(big_card(uid, name, BIG).as_str());
            create.send().promise.await.unwrap();
        }

        let (mut seen, mut sizes) = (vec![], vec![]);
        let mut cursor: Vec<u8> = vec![];
        loop {
            let mut req = persons.list_request();
            req.get().init_cursor().set_opaque(&cursor);
            let answer = req.send().promise.await.unwrap();
            let page = answer.get().unwrap().get_page().unwrap();
            let items = page.get_items().unwrap();
            sizes.push(items.len());
            seen.extend(items.iter().map(name_of));
            cursor = page.get_next().unwrap().get_opaque().unwrap().to_vec();
            if cursor.is_empty() {
                break;
            }
        }
        assert_eq!(seen, ["A", "B", "C"]);
        assert_eq!(sizes, [2, 1], "three would pass the weight of a page");

        let mut find = persons.find_by_email_request();
        find.get().set_email("same@x.org");
        expect_problem(find.send().promise.await, TOO_LARGE);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn what_could_not_come_back_is_not_taken() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let too_much = "x".repeat(MAX_INTAKE_BYTES + 1);

        let mut create = persons.create_request();
        create
            .get()
            .set_jscontact(big_card("u", "A", MAX_INTAKE_BYTES).as_str());
        expect_problem(create.send().promise.await, TOO_LARGE);

        let mut import = persons.import_text_request();
        import.get().set_media_type("text/vcard");
        import.get().set_text(too_much.as_str());
        expect_problem(import.send().promise.await, TOO_LARGE);

        let mut config = roles.load_config_request();
        config.get().set_json(too_much.as_str());
        expect_problem(config.send().promise.await, TOO_LARGE);

        // A card below the limit of a request that re-serialises above the core's limit, and a card
        // that grows by small edits, are the core's to refuse; the connection stays.
        let mut create = persons.create_request();
        create
            .get()
            .set_jscontact(big_card("u", "A", MAX_INTAKE_BYTES - 200).as_str());
        let answer = create.send().promise.await.unwrap();
        let person = answer.get().unwrap().get_person().unwrap();
        let id = common::id_of(person);
        let mut relate = persons.relate_request();
        fs_interfaces::rpc::write_uuid(relate.get().init_id(), Some(id));
        relate.get().set_other_uid("x".repeat(1000).as_str());
        relate.get().set_relation("friend");
        expect_problem(relate.send().promise.await, TOO_LARGE);

        let answer = persons.list_request().send().promise.await.unwrap();
        let page = answer.get().unwrap().get_page().unwrap();
        assert_eq!(page.get_items().unwrap().len(), 1);
    })
    .await;
}
