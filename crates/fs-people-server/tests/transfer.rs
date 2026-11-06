//! `importText` and `exportText` over a real socket.

mod common;

use common::{CLUB, card, create, persons_of, problem_of, roles_of, uuids, with_server};
use fs_interfaces::rpc::write_uuid;

const VCF: &str = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:vcard-1\r\nFN:Ada Lovelace\r\nEMAIL:ada@x.org\r\nEND:VCARD\r\n";

#[tokio::test(flavor = "current_thread")]
async fn import_reports_created_skipped_and_failed() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let mut import = persons.import_text_request();
        import.get().set_media_type("Text/vCard; charset=utf-8");
        import.get().set_text(VCF);
        let answer = import.send().promise.await.unwrap();
        let report = answer.get().unwrap().get_report().unwrap();
        assert_eq!(uuids(report.get_created().unwrap()).len(), 1);

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
        assert_eq!(uuids(report.get_created().unwrap()).len(), 1);
        let skipped = report.get_skipped().unwrap();
        assert_eq!(skipped.get(0).unwrap().to_str().unwrap(), "vcard-1");
        let failed = report.get_failed().unwrap();
        assert_eq!(failed.len(), 1);
        assert!(failed.get(0).unwrap().to_str().unwrap().contains("bad-1"));

        let mut broken = persons.import_text_request();
        broken.get().set_media_type("application/jscontact+json");
        broken.get().set_text("{ not json");
        assert_eq!(problem_of(broken.send().promise.await).status, 422);
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
        let refused = problem_of(import.send().promise.await);
        assert!(refused.is_unsupported());
        assert!(
            refused.detail.starts_with("person.import:"),
            "{}",
            refused.detail
        );

        let mut export = persons.export_text_request();
        write_uuid(export.get().init_id(), Some(id));
        export.get().set_media_type("text/csv");
        let refused = problem_of(export.send().promise.await);
        assert!(refused.is_unsupported());
        assert!(
            refused.detail.starts_with("person.export:"),
            "{}",
            refused.detail
        );
    })
    .await;
}
