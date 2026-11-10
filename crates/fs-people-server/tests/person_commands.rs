//! The type catalog and the commands patch, check, suggest over a real socket, against the store-backed provider.

mod common;

use common::{
    INVALID, create, expect_problem, id_of, json_of, persons_of, problem_of, with_server,
};
use fs_interfaces::rpc::write_uuid;
use fs_interfaces::types_capnp::BasicType;

#[tokio::test(flavor = "current_thread")]
async fn the_server_reports_the_description_of_person() {
    with_server(|root| async move {
        let answer = root.types_request().send().promise.await.unwrap();
        let catalog = answer.get().unwrap().get_catalog().unwrap();
        let answer = catalog.describe_request().send().promise.await.unwrap();
        let types = answer.get().unwrap().get_types().unwrap();
        assert_eq!(types.len(), 1);
        let person = types.get(0);
        assert_eq!(person.get_name().unwrap().to_str().unwrap(), "Person");
        assert_eq!(person.get_program().unwrap().to_str().unwrap(), "fs-people");
        assert_eq!(
            person.get_standard().unwrap().to_str().unwrap(),
            "RFC 9553 Card"
        );
        let fields = person.get_fields().unwrap();
        let emails = fields
            .iter()
            .find(|f| f.get_name().unwrap().to_str().unwrap() == "emails")
            .expect("emails is a field");
        assert_eq!(emails.get_basic().unwrap(), BasicType::Email);
        assert!(emails.get_multiple());
        let commands: Vec<String> = person
            .get_commands()
            .unwrap()
            .iter()
            .map(|c| c.get_name().unwrap().to_string().unwrap())
            .collect();
        assert_eq!(
            commands,
            ["create", "change", "find", "delete", "check", "suggest"]
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn patch_changes_what_it_names_and_a_bad_patch_is_a_problem() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, "uid-p", "Ada", "ada@x.org").await;

        let mut patch = persons.patch_request();
        write_uuid(patch.get().init_id(), Some(id));
        patch
            .get()
            .set_patch(r#"{"name/full":"Ada King","emails/e1/label":"main"}"#);
        let done = patch.send().promise.await.unwrap();
        let person = done.get().unwrap().get_person().unwrap();
        assert_eq!(id_of(person), id);
        let content = json_of(person);
        assert_eq!(content["name"]["full"], "Ada King");
        assert_eq!(content["emails"]["e1"]["label"], "main");
        assert_eq!(content["emails"]["e1"]["address"], "ada@x.org");

        for bad in [
            r#"[1]"#,
            "not json",
            r#"{"nope/x":1}"#,
            r#"{"uid":"other"}"#,
        ] {
            let mut patch = persons.patch_request();
            write_uuid(patch.get().init_id(), Some(id));
            patch.get().set_patch(bad);
            let outcome = patch.send().promise.await;
            let problem = problem_of(outcome);
            assert!(
                problem.status == 422 || problem.status == 409,
                "{bad}: {}",
                problem.status
            );
        }
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn check_names_what_breaks_a_rule_and_leaves_the_card_alone() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, "uid-c", "Ada", "ada@x.org").await;
        let mut ask = persons.check_request();
        write_uuid(ask.get().init_id(), Some(id));
        let ok = ask.send().promise.await.unwrap();
        assert_eq!(ok.get().unwrap().get_problems().unwrap().len(), 0);

        let mut patch = persons.patch_request();
        write_uuid(patch.get().init_id(), Some(id));
        patch.get().set_patch(r#"{"emails/e1/address":"nonsense"}"#);
        patch.send().promise.await.unwrap();

        let mut ask = persons.check_request();
        write_uuid(ask.get().init_id(), Some(id));
        let bad = ask.send().promise.await.unwrap();
        let problems = bad.get().unwrap().get_problems().unwrap();
        assert_eq!(problems.len(), 1);
        assert_eq!(
            problems.get(0).get_path().unwrap().to_str().unwrap(),
            "/emails/e1"
        );
        assert!(
            !problems
                .get(0)
                .get_code()
                .unwrap()
                .to_str()
                .unwrap()
                .contains(' ')
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn suggest_completes_from_the_book_and_refuses_a_field_it_does_not_know() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        create(&persons, "uid-s1", "A", "a@example.org").await;
        create(&persons, "uid-s2", "B", "b@example.org").await;
        let mut ask = persons.suggest_request();
        ask.get().set_field("emails");
        ask.get().set_typed("ada");
        let got = ask.send().promise.await.unwrap();
        let got = got.get().unwrap().get_suggestions().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got.get(0).unwrap().to_str().unwrap(), "ada@example.org");

        let mut ask = persons.suggest_request();
        ask.get().set_field("nope");
        ask.get().set_typed("x");
        expect_problem(ask.send().promise.await, INVALID);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn change_takes_typed_values_field_by_field() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, "uid-ch", "Ada", "ada@x.org").await;
        let send = |emails: &'static [&'static str]| {
            let persons = persons.clone();
            async move {
                let mut req = persons.change_request();
                write_uuid(req.get().init_id(), Some(id));
                let edit = req.get().init_edit();
                let mut fields = edit.init_fields(1);
                let mut f = fields.reborrow().get(0);
                f.reborrow().init_path(1).set(0, "emails");
                f.set_basic(BasicType::Email);
                let mut values = f.init_values(u32::try_from(emails.len()).unwrap());
                for (i, e) in (0u32..).zip(emails) {
                    values.set(i, *e);
                }
                req.send().promise.await
            }
        };
        let done = send(&["ada@x.org", "second@y.org"]).await.unwrap();
        let content = json_of(done.get().unwrap().get_person().unwrap());
        assert_eq!(content["emails"].as_object().unwrap().len(), 2);
        assert_eq!(content["emails"]["e1"]["address"], "ada@x.org");
        // A value that breaks the rule of its basic type is a problem, and nothing is written.
        let bad = problem_of(send(&["nonsense"]).await);
        assert_eq!(bad.status, 422);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn change_of_a_read_only_field_is_refused() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, "uid-ro", "Ada", "ada@x.org").await;
        let mut req = persons.change_request();
        write_uuid(req.get().init_id(), Some(id));
        let mut fields = req.get().init_edit().init_fields(1);
        let mut f = fields.reborrow().get(0);
        f.reborrow().init_path(1).set(0, "uid");
        f.set_basic(BasicType::Text);
        f.init_values(1).set(0, "other");
        assert_eq!(problem_of(req.send().promise.await).status, 422);
    })
    .await;
}
