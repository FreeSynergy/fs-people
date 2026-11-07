//! The facts of a problem arrive as typed extensions, so a client reads them instead of a sentence.

mod common;

use common::{
    CONFLICT, INVALID, SENSITIVE, SENSITIVE_LINK, STORED_DATA, card, create, id_of, persons_of,
    problem_of, revision_now, roles_of, unknown_id, update_if, with_server, with_store,
};
use fs_interfaces::rpc::write_uuid;
use fs_store::{Envelope, StoredObject};

const SECRET: &str = r#"{"@type":"Card","version":"1.0","uid":"s","name":{"full":"Secret"},"freesynergy.net:sensitive":["pd:HealthRecord"]}"#;

#[tokio::test(flavor = "current_thread")]
async fn not_found_names_the_object_and_its_kind() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let gone = unknown_id();
        let mut delete = persons.delete_request();
        write_uuid(delete.get().init_id(), Some(gone));
        let problem = problem_of(delete.send().promise.await);
        assert_eq!(problem.ext("id"), Some(gone.to_string().as_str()));
        assert_eq!(problem.ext("kind"), Some("person"));

        let mut members = persons.list_members_request();
        write_uuid(members.get().init_list(), Some(gone));
        let problem = problem_of(members.send().promise.await);
        assert_eq!(problem.ext("id"), Some(gone.to_string().as_str()));
        assert_eq!(problem.ext("kind"), Some("list"));

        let mut undefine = roles_of(&root).await.undefine_request();
        undefine.get().set_id("wizard");
        let problem = problem_of(undefine.send().promise.await);
        assert_eq!(problem.ext("kind"), Some("role"));
        assert!(problem.ext("id").is_some_and(|id| id.ends_with("wizard")));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_changed_card_is_a_conflict_about_that_person() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let old = revision_now(&persons, id).await;
        let mut edit = persons.update_request();
        write_uuid(edit.get().init_id(), Some(id));
        edit.get()
            .set_jscontact(card("uid-1", "Ada L", "a@x.org").as_str());
        edit.send().promise.await.unwrap();
        let late = update_if(&persons, id, old, &card("uid-1", "Late", "a@x.org")).await;
        let problem = problem_of(late);
        assert_eq!(problem.problem_type, CONFLICT.1);
        assert_eq!(problem.ext("id"), Some(id.to_string().as_str()));
        assert_eq!(problem.ext("kind"), Some("person"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_sensitive_card_says_which_one() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let mut make = persons.create_request();
        make.get().set_jscontact(SECRET);
        let answer = make.send().promise.await.unwrap();
        let secret = id_of(answer.get().unwrap().get_person().unwrap());
        let mut export = persons.export_text_request();
        write_uuid(export.get().init_id(), Some(secret));
        export.get().set_media_type("text/vcard");
        let problem = problem_of(export.send().promise.await);
        assert_eq!(problem.problem_type, SENSITIVE.1);
        assert_eq!(problem.ext("id"), Some(secret.to_string().as_str()));
        assert_eq!(problem.ext("kind"), Some("person"));
        assert_eq!(problem.ext("who"), Some("s: Secret"));

        let other = create(&persons, "uid-2", "A. King", "ada@x.org").await;
        let mut confirm = persons.confirm_same_request();
        write_uuid(confirm.get().init_a(), Some(secret));
        write_uuid(confirm.get().init_b(), Some(other));
        let problem = problem_of(confirm.send().promise.await);
        assert_eq!(problem.problem_type, SENSITIVE_LINK.1);
        assert_eq!(problem.ext("id"), Some(secret.to_string().as_str()));
        assert_eq!(problem.ext("kind"), Some("person"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_gives_one_issue_per_reason_and_the_number_as_value() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let mut bad = persons.create_request();
        bad.get()
            .set_jscontact(r#"{"@type":"Card","version":"9","uid":"u","kind":"robot"}"#);
        let problem = problem_of(bad.send().promise.await);
        assert_eq!(problem.problem_type, INVALID.1);
        let issues: Vec<&str> = problem.ext_all("issue").collect();
        assert!(issues.len() >= 2, "{issues:?}");
        assert!(issues.iter().any(|i| i.contains("/version")), "{issues:?}");
        assert!(
            problem.detail.starts_with("card violates RFC 9553: "),
            "{}",
            problem.detail
        );

        let zero =
            r#"{"@type":"Card","version":"1.0","uid":"z","name":{"full":"N"},"example.org:n":-0}"#;
        let mut inexact = persons.create_request();
        inexact.get().set_jscontact(zero);
        let problem = problem_of(inexact.send().promise.await);
        assert_eq!(problem.problem_type, INVALID.1);
        assert_eq!(problem.ext("value"), Some("-0"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn stored_data_gives_its_reason_as_an_issue() {
    with_store(|root, store| async move {
        let persons = persons_of(&root).await;
        let envelope = Envelope::new("Profile", "Broken");
        let id = envelope.id;
        let object = StoredObject::new(
            envelope,
            "application/jscontact+json",
            serde_json::json!({"uid": 5}),
        );
        store.put(object, common::ME).await.unwrap();
        let mut read = persons.relations_request();
        write_uuid(read.get().init_id(), Some(id.as_uuid()));
        let problem = problem_of(read.send().promise.await);
        assert_eq!(problem.problem_type, STORED_DATA.1);
        let reason = problem.ext("issue").expect("a reason");
        assert!(
            problem.detail.ends_with(reason),
            "{} / {reason}",
            problem.detail
        );
    })
    .await;
}
