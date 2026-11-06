//! A sensitive marking of any shape against the real program, whose store is not encrypted.

#[path = "common/program.rs"]
mod program;

use fs_interfaces::people_capnp::people;
use fs_interfaces::rpc::{Problem, connect, read_uuid, write_uuid};
use program::{start_with, terminate, wait_for};
use serde_json::json;

/// `[]`, `true`, `{}`, `[1]`, `null`: a marking that names no category still counts, here as in the
/// store, which refuses it without encryption.
#[tokio::test(flavor = "current_thread")]
async fn a_marking_of_any_shape_meets_the_encryption_refusal_of_the_real_program() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let socket = dir.path().join("people.sock");
            let server = start_with(&socket, &dir.path().join("data"), &["--instance", "example.org"]);
            wait_for(&socket).await;
            let connection = connect::<people::Client>(&socket).await.unwrap();
            let answer = connection.client.persons_request().send().promise.await.unwrap();
            let persons = answer.get().unwrap().get_persons().unwrap();
            let problem = |e: capnp::Error| Problem::from_error(&e).unwrap();

            let plain = json!({"@type": "Card", "version": "1.0", "uid": "plain", "name": {"full": "Plain"}});
            let mut create = persons.create_request();
            create.get().set_jscontact(plain.to_string().as_str());
            let answer = create.send().promise.await.unwrap();
            let envelope = answer.get().unwrap().get_person().unwrap().get_envelope().unwrap();
            let id = read_uuid(envelope.get_id().unwrap());
            let iri = envelope.get_iri().unwrap().to_str().unwrap();
            assert!(iri.starts_with("https://example.org/people/"), "{iri}");

            for (i, mark) in [json!([]), json!(true), json!({}), json!([1]), json!(null)].iter().enumerate() {
                let marked = |uid: String| {
                    json!({"@type": "Card", "version": "1.0", "uid": uid, "name": {"full": "M"},
                        "freesynergy.net:sensitive": mark}).to_string()
                };
                let mut create = persons.create_request();
                create.get().set_jscontact(marked(format!("new-{i}")).as_str());
                let refused = problem(create.send().promise.await.err().unwrap());
                assert_eq!(refused.problem_type, "https://freesynergy.net/problems/people/invalid", "{mark}");

                let mut import = persons.import_text_request();
                import.get().set_media_type("application/jscontact+json");
                import.get().set_text(marked(format!("imp-{i}")).as_str());
                let answer = import.send().promise.await.unwrap();
                let report = answer.get().unwrap().get_report().unwrap();
                assert_eq!(report.get_created().unwrap().len(), 0, "{mark}");
                let failed = report.get_failed().unwrap();
                assert!(failed.get(0).unwrap().to_str().unwrap().contains("not encrypted"), "{mark}");

                let mut update = persons.update_request();
                write_uuid(update.get().init_id(), id);
                update.get().set_jscontact(marked("plain".into()).as_str());
                let refused = problem(update.send().promise.await.err().unwrap());
                assert_eq!(refused.problem_type, "https://freesynergy.net/problems/people/encryption-required", "{mark}");
            }
            drop(connection);
            terminate(server);
        })
        .await;
}
