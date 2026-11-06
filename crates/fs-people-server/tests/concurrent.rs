//! Calls at the same time, as several programs on one socket make them: no change gets lost.

mod common;

use common::{card, create, id_of, json_of, persons_of, roles_of, with_server};
use fs_interfaces::rpc::write_uuid;

const CLUB: &str = common::CLUB;

#[tokio::test(flavor = "current_thread")]
async fn members_added_at_the_same_time_all_stay() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let mut make = persons.create_list_request();
        make.get().set_name("Everyone");
        let answer = make.send().promise.await.unwrap();
        let list = id_of(answer.get().unwrap().get_person().unwrap());
        let mut members = vec![];
        for i in 0..20 {
            members.push(create(&persons, &format!("uid-{i}"), "Member", "m@x.org").await);
        }

        let mut calls = tokio::task::JoinSet::new();
        for member in members {
            let mut add = persons.add_to_list_request();
            write_uuid(add.get().init_list(), Some(list));
            write_uuid(add.get().init_member(), Some(member));
            calls.spawn_local(add.send().promise);
        }
        while let Some(done) = calls.join_next().await {
            assert!(done.unwrap().is_ok());
        }

        let mut get = persons.get_request();
        write_uuid(get.get().init_id(), Some(list));
        let answer = get.send().promise.await.unwrap();
        let card = json_of(answer.get().unwrap().get_person().unwrap());
        assert_eq!(card["members"].as_object().unwrap().len(), 20);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn one_uid_is_taken_once_and_one_role_held_once() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let mut calls = tokio::task::JoinSet::new();
        for _ in 0..8 {
            let mut make = persons.create_request();
            make.get().set_jscontact(card("uid-1", "Ada", "a@x.org").as_str());
            calls.spawn_local(make.send().promise);
        }
        let mut created = vec![];
        while let Some(done) = calls.join_next().await {
            if let Ok(answer) = done.unwrap() {
                created.push(id_of(answer.get().unwrap().get_person().unwrap()));
            }
        }
        assert_eq!(created.len(), 1, "one uid, one card");

        let mut calls = tokio::task::JoinSet::new();
        for _ in 0..8 {
            let mut assign = roles.assign_request();
            write_uuid(assign.get().init_person(), Some(created[0]));
            assign.get().set_role("member");
            assign.get().set_scope(CLUB);
            calls.spawn_local(assign.send().promise);
        }
        while let Some(done) = calls.join_next().await {
            assert!(done.unwrap().is_ok());
        }
        let mut of = roles.roles_of_request();
        write_uuid(of.get().init_person(), Some(created[0]));
        let answer = of.send().promise.await.unwrap();
        assert_eq!(answer.get().unwrap().get_items().unwrap().len(), 1);
    })
    .await;
}
