//! `importText` and `exportText` over a real socket.

mod common;

use common::{
    CLUB, INVALID, NOT_POSSIBLE, card, create, expect_problem, id_of, name_of, persons_of,
    roles_of, uuids, with_server,
};
use fs_interfaces::rpc::write_uuid;

const VCF: &str = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:vcard-1\r\nFN:Ada Lovelace\r\nEMAIL:ada@x.org\r\nEND:VCARD\r\n";

async fn id_by_uid(
    persons: &fs_interfaces::people_capnp::persons::Client,
    uid: &str,
) -> uuid::Uuid {
    let mut get = persons.get_by_uid_request();
    get.get().set_uid(uid);
    let answer = get.send().promise.await.unwrap();
    assert!(answer.get().unwrap().get_found(), "{uid}");
    id_of(answer.get().unwrap().get_person().unwrap())
}

fn texts(list: capnp::text_list::Reader<'_>) -> Vec<String> {
    list.iter()
        .map(|t| t.unwrap().to_string().unwrap())
        .collect()
}

#[tokio::test(flavor = "current_thread")]
async fn import_reports_created_skipped_and_failed() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let mut import = persons.import_text_request();
        import.get().set_media_type("Text/vCard; charset=utf-8");
        import.get().set_text(VCF);
        let answer = import.send().promise.await.unwrap();
        let report = answer.get().unwrap().get_report().unwrap();
        let created = uuids(report.get_created().unwrap());
        assert_eq!(created, [id_by_uid(&persons, "vcard-1").await]);
        assert!(texts(report.get_skipped().unwrap()).is_empty());
        assert!(texts(report.get_failed().unwrap()).is_empty());

        let two = format!(
            "[{},{},{}]",
            card("vcard-1", "Known", "k@x.org"),
            card("new-1", "New", "n@x.org"),
            r#"{"@type":"Card","version":"9","uid":"bad-1"}"#,
        );
        let mut import = persons.import_text_request();
        import.get().set_media_type("application/jscontact+json");
        import.get().set_text(two.as_str());
        let answer = import.send().promise.await.unwrap();
        let report = answer.get().unwrap().get_report().unwrap();
        let created = uuids(report.get_created().unwrap());
        assert_eq!(created, [id_by_uid(&persons, "new-1").await]);
        assert_eq!(texts(report.get_skipped().unwrap()), ["vcard-1"]);
        let failed = texts(report.get_failed().unwrap());
        assert_eq!(failed.len(), 1);
        assert!(
            failed[0].starts_with("bad-1:") && failed[0].contains("/version"),
            "{failed:?}"
        );

        // A known uid was not overwritten.
        let mut get = persons.get_by_uid_request();
        get.get().set_uid("vcard-1");
        let answer = get.send().promise.await.unwrap();
        assert_eq!(
            name_of(answer.get().unwrap().get_person().unwrap()),
            "Ada Lovelace"
        );

        // What is read nowhere is reported, too.
        let mut lost = persons.import_text_request();
        lost.get().set_media_type("text/vcard");
        lost.get().set_text("this is no vCard");
        let answer = lost.send().promise.await.unwrap();
        let report = answer.get().unwrap().get_report().unwrap();
        assert!(uuids(report.get_created().unwrap()).is_empty());
        let failed = texts(report.get_failed().unwrap());
        assert_eq!(failed.len(), 1);
        assert!(
            failed[0].contains("outside any complete vCard"),
            "{failed:?}"
        );

        let mut broken = persons.import_text_request();
        broken.get().set_media_type("application/jscontact+json");
        broken.get().set_text("{ not json");
        expect_problem(broken.send().promise.await, INVALID);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn export_writes_the_format_asked_for_and_never_a_role() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let id = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let mut assign = roles.assign_request();
        write_uuid(assign.get().init_person(), Some(id));
        assign.get().set_role("member");
        assign.get().set_scope(CLUB);
        assign.send().promise.await.unwrap();

        for (media_type, marker) in [
            ("text/vcard", "BEGIN:VCARD"),
            ("application/jscontact+json", "\"@type\""),
        ] {
            let mut export = persons.export_text_request();
            write_uuid(export.get().init_id(), Some(id));
            export.get().set_media_type(media_type);
            let answer = export.send().promise.await.unwrap();
            let text = answer.get().unwrap().get_text().unwrap().to_str().unwrap();
            assert!(
                text.contains(marker) && text.contains("ada@x.org"),
                "{text}"
            );
            assert!(!text.contains("member") && !text.contains("club"), "{text}");
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_media_type_without_a_format_is_not_possible() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, "uid-1", "Ada", "ada@x.org").await;

        let mut import = persons.import_text_request();
        import.get().set_media_type("text/csv");
        import.get().set_text("a,b");
        let detail = expect_problem(import.send().promise.await, NOT_POSSIBLE);
        assert!(detail.starts_with("person.import:"), "{detail}");

        let mut export = persons.export_text_request();
        write_uuid(export.get().init_id(), Some(id));
        export.get().set_media_type("text/csv");
        let detail = expect_problem(export.send().promise.await, NOT_POSSIBLE);
        assert!(detail.starts_with("person.export:"), "{detail}");
    })
    .await;
}

const UUID: &str = "9f8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d";

#[tokio::test(flavor = "current_thread")]
async fn the_other_spelling_of_a_uid_is_skipped_not_failed() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, &format!("urn:uuid:{UUID}"), "Ada", "ada@x.org").await;
        let mut import = persons.import_text_request();
        import.get().set_media_type("application/jscontact+json");
        let upper = UUID.to_uppercase();
        import
            .get()
            .set_text(card(&upper, "Again", "a@x.org").as_str());
        let answer = import.send().promise.await.unwrap();
        let report = answer.get().unwrap().get_report().unwrap();
        assert_eq!(
            texts(report.get_skipped().unwrap()),
            std::slice::from_ref(&upper)
        );
        assert!(texts(report.get_failed().unwrap()).is_empty());
        assert!(uuids(report.get_created().unwrap()).is_empty());
        assert_eq!(id_by_uid(&persons, UUID).await, id);
    })
    .await;
}
