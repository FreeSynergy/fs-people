//! Failures and the provider's report, as a consumer sees them.

mod common;

use common::{
    BAD_REQUEST, CLUB, CONFLICT, INVALID, ME, NOT_FOUND, STORED_DATA, card, create, expect_problem,
    id_of, json_of, persons_of, roles_of, unknown_id, with_server, with_store,
};
use fs_interfaces::rpc::write_uuid;
use fs_objects::{Envelope, StoredObject};
use fs_people_server::CONTRACT_VERSION;
use serde_json::json;

#[tokio::test(flavor = "current_thread")]
async fn errors_arrive_typed() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let id = create(&persons, "uid-1", "Ada", "ada@x.org").await;

        let mut twice = persons.create_request();
        twice
            .get()
            .set_jscontact(card("uid-1", "Again", "a@x.org").as_str());
        expect_problem(twice.send().promise.await, CONFLICT);

        let mut broken = persons.create_request();
        broken.get().set_jscontact("{ not json");
        expect_problem(broken.send().promise.await, BAD_REQUEST);

        let mut against_the_rfc = persons.create_request();
        against_the_rfc
            .get()
            .set_jscontact(r#"{"@type":"Card","version":"9","uid":"u"}"#);
        let detail = expect_problem(against_the_rfc.send().promise.await, INVALID);
        assert!(detail.contains("/version"), "{detail}");

        let mut other_uid = persons.update_request();
        write_uuid(other_uid.get().init_id(), Some(id));
        other_uid
            .get()
            .set_jscontact(card("uid-other", "Ada", "a@x.org").as_str());
        expect_problem(other_uid.send().promise.await, CONFLICT);

        let mut unknown = persons.delete_request();
        write_uuid(unknown.get().init_id(), Some(unknown_id()));
        expect_problem(unknown.send().promise.await, NOT_FOUND);

        let no_id = persons.delete_request();
        expect_problem(no_id.send().promise.await, BAD_REQUEST);

        let mut bad_cursor = persons.list_request();
        bad_cursor.get().init_cursor().set_opaque(&[9, 9]);
        expect_problem(bad_cursor.send().promise.await, BAD_REQUEST);

        let mut unknown_role = roles.assign_request();
        write_uuid(unknown_role.get().init_person(), Some(id));
        unknown_role.get().set_role("wizard");
        expect_problem(unknown_role.send().promise.await, CONFLICT);

        let mut not_an_object = roles.assign_request();
        write_uuid(not_an_object.get().init_person(), Some(id));
        not_an_object.get().set_role("ai");
        not_an_object.get().set_details("[1]");
        expect_problem(not_an_object.send().promise.await, BAD_REQUEST);

        let mut not_an_iri = roles.assign_request();
        write_uuid(not_an_iri.get().init_person(), Some(id));
        not_an_iri.get().set_role("member");
        not_an_iri.get().set_scope("personal");
        expect_problem(not_an_iri.send().promise.await, INVALID);

        let mut foreign_fields = roles.assign_request();
        write_uuid(foreign_fields.get().init_person(), Some(id));
        foreign_fields.get().set_role("member");
        foreign_fields.get().set_scope(CLUB);
        foreign_fields.get().set_details(r#"{"a":1}"#);
        expect_problem(foreign_fields.send().promise.await, INVALID);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_empty_value_is_a_malformed_request() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let id = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let by_uid = persons.get_by_uid_request();
        expect_problem(by_uid.send().promise.await, BAD_REQUEST);
        let email = persons.find_by_email_request();
        expect_problem(email.send().promise.await, BAD_REQUEST);
        let phone = persons.find_by_phone_request();
        expect_problem(phone.send().promise.await, BAD_REQUEST);
        let iri = persons.find_by_describes_request();
        expect_problem(iri.send().promise.await, BAD_REQUEST);
        let name = persons.create_list_request();
        expect_problem(name.send().promise.await, BAD_REQUEST);
        let mut relation = persons.relate_request();
        write_uuid(relation.get().init_id(), Some(id));
        relation.get().set_other_uid("uid-2");
        expect_problem(relation.send().promise.await, BAD_REQUEST);
        let mut media_type = persons.export_text_request();
        write_uuid(media_type.get().init_id(), Some(id));
        expect_problem(media_type.send().promise.await, BAD_REQUEST);
        let role = roles.get_request();
        expect_problem(role.send().promise.await, BAD_REQUEST);
        let config = roles.load_config_request();
        expect_problem(config.send().promise.await, BAD_REQUEST);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn broken_stored_data_is_not_the_callers_fault_and_a_panic_ends_nothing() {
    with_store(|root, store| async move {
        let persons = persons_of(&root).await;
        let mut envelope = Envelope::new("Profile", "Broken");
        envelope.context = Some("people".into());
        let id = envelope.id;
        let object = StoredObject::new(envelope, "application/jscontact+json", json!({"uid": 5}));
        store.put(object, ME).await.unwrap();
        let mut read = persons.relations_request();
        write_uuid(read.get().init_id(), Some(id.as_uuid()));
        expect_problem(read.send().promise.await, STORED_DATA);

        // A value `relatedTo` may hold, which an edit once panicked on and took the connection along.
        let text = json!({"@type": "Card", "version": "1.0", "uid": "odd", "name": {"full": "Odd"},
            "relatedTo": {"x": "not an object"}});
        let mut create = persons.create_request();
        create.get().set_jscontact(text.to_string().as_str());
        let answer = create.send().promise.await.unwrap();
        let odd = id_of(answer.get().unwrap().get_person().unwrap());
        let mut relate = persons.relate_request();
        write_uuid(relate.get().init_id(), Some(odd));
        relate.get().set_other_uid("x");
        relate.get().set_relation("friend");
        let answer = relate.send().promise.await.unwrap();
        let card = json_of(answer.get().unwrap().get_person().unwrap());
        assert_eq!(card["relatedTo"]["x"]["relation"]["friend"], true);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_report_names_every_capability_of_the_contract() {
    with_server(|root| async move {
        let answer = root.support_report_request().send().promise.await.unwrap();
        let report = answer.get().unwrap().get_report().unwrap();
        assert_eq!(
            report.get_contract_version().unwrap().to_str().unwrap(),
            CONTRACT_VERSION
        );
        assert_eq!(
            report.get_provider().unwrap().to_str().unwrap(),
            "fs-people"
        );
        let mut ids: Vec<String> = report
            .get_entries()
            .unwrap()
            .iter()
            .inspect(|e| assert!(e.get_supported()))
            .map(|e| e.get_capability().unwrap().to_string().unwrap())
            .collect();
        ids.sort();
        assert_eq!(
            ids,
            [
                "ai.mark",
                "cardgroup.read",
                "cardgroup.write",
                "person.check",
                "person.create",
                "person.delete",
                "person.export",
                "person.find",
                "person.import",
                "person.read",
                "person.subscribe",
                "person.suggest",
                "person.update",
                "relation.read",
                "relation.write",
                "role.assign",
                "role.configure",
                "role.define",
                "role.read",
                "same.link",
                "same.propose",
                "same.read",
            ]
        );
    })
    .await;
}
