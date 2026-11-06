//! Failures and the provider's report, as a consumer sees them.

mod common;

use common::{CLUB, card, create, persons_of, problem_of, roles_of, unknown_id, with_server};
use fs_interfaces::rpc::write_uuid;
use fs_people_server::CONTRACT_VERSION;

const PROBLEMS: &str = "https://freesynergy.net/problems/people/";

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
        let e = problem_of(twice.send().promise.await);
        assert_eq!(
            (e.status, e.problem_type),
            (409, format!("{PROBLEMS}conflict"))
        );

        let mut broken = persons.create_request();
        broken.get().set_jscontact("{ not json");
        let e = problem_of(broken.send().promise.await);
        assert_eq!(
            (e.status, e.problem_type),
            (422, format!("{PROBLEMS}invalid"))
        );

        let mut against_the_rfc = persons.create_request();
        against_the_rfc
            .get()
            .set_jscontact(r#"{"@type":"Card","version":"9","uid":"u"}"#);
        assert_eq!(problem_of(against_the_rfc.send().promise.await).status, 422);

        let mut other_uid = persons.update_request();
        write_uuid(other_uid.get().init_id(), Some(id));
        other_uid
            .get()
            .set_jscontact(card("uid-other", "Ada", "a@x.org").as_str());
        assert_eq!(problem_of(other_uid.send().promise.await).status, 409);

        let mut unknown = persons.delete_request();
        write_uuid(unknown.get().init_id(), Some(unknown_id()));
        let e = problem_of(unknown.send().promise.await);
        assert_eq!(
            (e.status, e.problem_type),
            (404, format!("{PROBLEMS}not-found"))
        );

        let no_id = persons.delete_request();
        assert_eq!(problem_of(no_id.send().promise.await).status, 422);

        let mut bad_cursor = persons.list_request();
        bad_cursor.get().init_cursor().set_opaque(&[9, 9]);
        assert_eq!(problem_of(bad_cursor.send().promise.await).status, 422);

        let mut unknown_role = roles.assign_request();
        write_uuid(unknown_role.get().init_person(), Some(id));
        unknown_role.get().set_role("wizard");
        assert_eq!(problem_of(unknown_role.send().promise.await).status, 409);

        let mut not_an_object = roles.assign_request();
        write_uuid(not_an_object.get().init_person(), Some(id));
        not_an_object.get().set_role("ai");
        not_an_object.get().set_details("[1]");
        assert_eq!(problem_of(not_an_object.send().promise.await).status, 422);

        let mut not_an_iri = roles.assign_request();
        write_uuid(not_an_iri.get().init_person(), Some(id));
        not_an_iri.get().set_role("member");
        not_an_iri.get().set_scope("personal");
        assert_eq!(problem_of(not_an_iri.send().promise.await).status, 422);

        let mut foreign_fields = roles.assign_request();
        write_uuid(foreign_fields.get().init_person(), Some(id));
        foreign_fields.get().set_role("member");
        foreign_fields.get().set_scope(CLUB);
        foreign_fields.get().set_details(r#"{"a":1}"#);
        assert_eq!(problem_of(foreign_fields.send().promise.await).status, 422);
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
                "person.create",
                "person.delete",
                "person.export",
                "person.find",
                "person.import",
                "person.read",
                "person.subscribe",
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
