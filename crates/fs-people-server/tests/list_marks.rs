//! Sensitive members of a list over the wire (ADR 0029).

mod common;

use common::{expect_problem, id_of, persons_of, with_server};
use fs_interfaces::rpc::{read_envelope, write_uuid};

#[tokio::test(flavor = "current_thread")]
async fn a_list_carries_the_mark_and_does_not_export_a_sensitive_member() {
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

        let mut export = persons.export_text_request();
        write_uuid(export.get().init_id(), Some(list));
        export.get().set_media_type("text/vcard");
        let refused = expect_problem(
            export.send().promise.await,
            (403, "https://freesynergy.net/problems/people/sensitive-members"),
        );
        assert!(refused.contains("s: Secret"), "{refused}");
    })
    .await;
}
