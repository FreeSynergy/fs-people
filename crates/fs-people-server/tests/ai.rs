//! Marking an AI (ADR 0054) over a real socket.

mod common;

use common::{CONFLICT, create, expect_problem, id_of, json_of, persons_of, roles_of, with_server};
use fs_interfaces::rpc::write_uuid;

#[tokio::test(flavor = "current_thread")]
async fn ai_marking_shows_in_kind_and_role() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let ada = create(&persons, "uid-1", "Ada", "ada@x.org").await;

        let mut is_ai = roles.is_ai_request();
        write_uuid(is_ai.get().init_person(), Some(ada));
        let answer = is_ai.send().promise.await.unwrap();
        assert!(!answer.get().unwrap().get_ai());

        let mut mark = roles.mark_ai_request();
        write_uuid(mark.get().init_person(), Some(ada));
        let answer = mark.send().promise.await.unwrap();
        let held = answer.get().unwrap().get_assignment().unwrap();
        assert_eq!(held.get_role().unwrap().to_str().unwrap(), "ai");

        let mut is_ai = roles.is_ai_request();
        write_uuid(is_ai.get().init_person(), Some(ada));
        let answer = is_ai.send().promise.await.unwrap();
        assert!(answer.get().unwrap().get_ai());

        let mut get = persons.get_request();
        write_uuid(get.get().init_id(), Some(ada));
        let answer = get.send().promise.await.unwrap();
        let person = answer.get().unwrap().get_person().unwrap();
        assert_eq!(json_of(person)["kind"], "application");
        id_of(person); // the role itself is not on the Person
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_ai_cannot_be_updated_out_of_its_kind() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let ada = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let mut mark = roles.mark_ai_request();
        write_uuid(mark.get().init_person(), Some(ada));
        mark.send().promise.await.unwrap();

        let mut update = persons.update_request();
        write_uuid(update.get().init_id(), Some(ada));
        update
            .get()
            .set_jscontact(common::card("uid-1", "Ada", "ada@x.org").as_str());
        expect_problem(update.send().promise.await, CONFLICT);

        let mut is_ai = roles.is_ai_request();
        write_uuid(is_ai.get().init_person(), Some(ada));
        let answer = is_ai.send().promise.await.unwrap();
        assert!(answer.get().unwrap().get_ai());
    })
    .await;
}
