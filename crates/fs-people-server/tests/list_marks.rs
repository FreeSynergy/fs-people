//! Sensitive members of a list over the wire (ADR 0029).

mod common;

use common::{SENSITIVE, expect_problem, id_of, persons_of, with_server};
use fs_interfaces::rpc::{read_envelope, write_uuid};

#[tokio::test(flavor = "current_thread")]
async fn a_list_carries_the_mark_and_nothing_sensitive_leaves_without_release() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let text = r#"{"@type":"Card","version":"1.0","uid":"s","name":{"full":"Secret"},"freesynergy.net:sensitive":["pd:HealthRecord"]}"#;
        let mut make = persons.create_request();
        make.get().set_jscontact(text);
        let answer = make.send().promise.await.unwrap();
        let secret = id_of(answer.get().unwrap().get_person().unwrap());
        let mut make = persons.create_list_request();
        make.get().set_name("L");
        let answer = make.send().promise.await.unwrap();
        let list = id_of(answer.get().unwrap().get_person().unwrap());
        let mut add = persons.add_to_list_request();
        write_uuid(add.get().init_list(), Some(list));
        write_uuid(add.get().init_member(), Some(secret));
        add.send().promise.await.unwrap();

        let mut members = persons.list_members_request();
        write_uuid(members.get().init_list(), Some(list));
        let answer = members.send().promise.await.unwrap();
        let items = answer.get().unwrap().get_items().unwrap();
        let envelope = read_envelope(items.get(0).get_envelope().unwrap()).unwrap();
        assert_eq!(envelope.sensitive, ["pd:HealthRecord"]);

        // The list leaves without them, and says who stayed behind.
        let mut export = persons.export_text_request();
        write_uuid(export.get().init_id(), Some(list));
        export.get().set_media_type("application/jscontact+json");
        let answer = export.send().promise.await.unwrap();
        let r = answer.get().unwrap();
        assert!(!r.get_text().unwrap().to_str().unwrap().contains("\"s\""));
        let withheld = r.get_withheld().unwrap();
        assert_eq!(withheld.len(), 1);
        assert_eq!(withheld.get(0).unwrap().to_str().unwrap(), "s: Secret");

        // On release, it goes along.
        let mut export = persons.export_text_request();
        write_uuid(export.get().init_id(), Some(list));
        export.get().set_media_type("application/jscontact+json");
        export.get().init_released(1).set(0, "s");
        let answer = export.send().promise.await.unwrap();
        let r = answer.get().unwrap();
        assert!(r.get_text().unwrap().to_str().unwrap().contains("\"s\""));
        assert_eq!(r.get_withheld().unwrap().len(), 0);

        // The card itself: refused, and on release its text.
        let mut export = persons.export_text_request();
        write_uuid(export.get().init_id(), Some(secret));
        export.get().set_media_type("text/vcard");
        let refused = expect_problem(export.send().promise.await, SENSITIVE);
        assert!(refused.contains("Secret"), "{refused}");
        let mut export = persons.export_text_request();
        write_uuid(export.get().init_id(), Some(secret));
        export.get().set_media_type("text/vcard");
        export.get().init_released(1).set(0, "s");
        let answer = export.send().promise.await.unwrap();
        assert!(answer.get().unwrap().get_text().unwrap().to_str().unwrap().contains("Secret"));
    })
    .await;
}
