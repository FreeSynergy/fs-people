//! What `Roles` refuses and keeps, over a real socket.

mod common;

use common::{
    BAD_REQUEST, CONFLICT, INVALID, NOT_FOUND, create, expect_problem, persons_of, roles_of, uuids,
    with_server,
};
use fs_interfaces::people_capnp::RoleScope;
use fs_interfaces::rpc::write_uuid;

#[tokio::test(flavor = "current_thread")]
async fn what_a_personal_role_and_a_held_one_answer() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let (ada, bob) = (
            create(&persons, "uid-1", "Ada", "ada@x.org").await,
            create(&persons, "uid-2", "Bob", "bob@x.org").await,
        );
        let mut define = roles.define_request();
        let mut d = define.get().init_definition();
        d.set_id("coach");
        d.reborrow()
            .init_names(1)
            .reborrow()
            .get(0)
            .set_text("Coach");
        d.set_owner("fs-people");
        d.set_scope(RoleScope::Personal);
        define.send().promise.await.unwrap();

        let mut assign = roles.assign_request();
        write_uuid(assign.get().init_person(), Some(ada));
        assign.get().set_role("coach");
        assign.get().set_details(r#"{"n":1.5}"#);
        let answer = assign.send().promise.await.unwrap();
        let held = answer.get().unwrap().get_assignment().unwrap();
        assert_eq!(
            held.get_details().unwrap().to_str().unwrap(),
            r#"{"n":1.5}"#
        );

        let mut too_long = roles.assign_request();
        write_uuid(too_long.get().init_person(), Some(bob));
        too_long.get().set_role("coach");
        too_long
            .get()
            .set_details(r#"{"n":123456789012345678901234567890}"#);
        let detail = expect_problem(too_long.send().promise.await, BAD_REQUEST);
        assert!(detail.contains("kept exactly"), "{detail}");

        let mut again = roles.assign_request();
        write_uuid(again.get().init_person(), Some(ada));
        again.get().set_role("coach");
        again.get().set_details(r#"{"n":1}"#);
        expect_problem(again.send().promise.await, CONFLICT);

        let mut group_needed = roles.assign_request();
        write_uuid(group_needed.get().init_person(), Some(bob));
        group_needed.get().set_role("member");
        expect_problem(group_needed.send().promise.await, INVALID);

        let mut with = roles.people_with_request();
        with.get().set_role("coach");
        let answer = with.send().promise.await.unwrap();
        assert_eq!(uuids(answer.get().unwrap().get_ids().unwrap()), [ada]);

        let mut unknown = roles.undefine_request();
        unknown.get().set_id("nobody");
        expect_problem(unknown.send().promise.await, NOT_FOUND);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn each_reach_of_a_role_travels_as_itself() {
    with_server(|root| async move {
        let roles = roles_of(&root).await;
        for (id, scope) in [
            ("a", RoleScope::Personal),
            ("b", RoleScope::Group),
            ("c", RoleScope::Public),
        ] {
            let mut define = roles.define_request();
            let mut d = define.get().init_definition();
            d.set_id(id);
            d.reborrow().init_names(1).reborrow().get(0).set_text(id);
            d.set_owner("o");
            d.set_scope(scope);
            define.send().promise.await.unwrap();
            let mut get = roles.get_request();
            get.get().set_id(id);
            let answer = get.send().promise.await.unwrap();
            let found = answer.get().unwrap().get_definition().unwrap();
            assert_eq!(found.get_scope().unwrap(), scope, "{id}");
        }
    })
    .await;
}
