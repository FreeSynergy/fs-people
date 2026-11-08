//! Relations, same-person links and lists of people over a real socket.

mod common;

use common::{CONFLICT, create, expect_problem, id_of, json_of, persons_of, uuids, with_server};
use fs_interfaces::people_capnp::SameState;
use fs_interfaces::rpc::{read_uuid, write_uuid};

#[tokio::test(flavor = "current_thread")]
async fn relations_stay_on_the_card() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let id = create(&persons, "uid-1", "Ada", "ada@x.org").await;

        let mut relate = persons.relate_request();
        write_uuid(relate.get().init_id(), Some(id));
        relate.get().set_other_uid("uid-2");
        relate.get().set_relation("friend");
        let answer = relate.send().promise.await.unwrap();
        let card = json_of(answer.get().unwrap().get_person().unwrap());
        assert_eq!(card["relatedTo"]["uid-2"]["relation"]["friend"], true);

        let mut relations = persons.relations_request();
        write_uuid(relations.get().init_id(), Some(id));
        let answer = relations.send().promise.await.unwrap();
        let items = answer.get().unwrap().get_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items.get(0).get_uid().unwrap().to_str().unwrap(), "uid-2");
        assert_eq!(
            items.get(0).get_relation().unwrap().to_str().unwrap(),
            "friend"
        );

        let mut unrelate = persons.unrelate_request();
        write_uuid(unrelate.get().init_id(), Some(id));
        unrelate.get().set_other_uid("uid-2");
        unrelate.get().set_relation("friend");
        let answer = unrelate.send().promise.await.unwrap();
        let card = json_of(answer.get().unwrap().get_person().unwrap());
        assert!(card.get("relatedTo").is_none());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn proposals_link_and_unlink() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let a = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let b = create(&persons, "uid-2", "A. King", "ada@x.org").await;
        create(&persons, "uid-3", "Bob", "bob@x.org").await;

        let answer = persons.proposals_request().send().promise.await.unwrap();
        let items = answer.get().unwrap().get_items().unwrap();
        assert_eq!(items.len(), 1);
        let proposal = items.get(0);
        let mut pair = [
            read_uuid(proposal.get_a().unwrap()).unwrap(),
            read_uuid(proposal.get_b().unwrap()).unwrap(),
        ];
        pair.sort_unstable();
        assert_eq!(pair, [a, b]);
        let reason = proposal.get_reasons().unwrap().get(0).unwrap();
        assert_eq!(reason.to_str().unwrap(), "email:ada@x.org");

        let mut confirm = persons.confirm_same_request();
        write_uuid(confirm.get().init_a(), Some(a));
        write_uuid(confirm.get().init_b(), Some(b));
        let answer = confirm.send().promise.await.unwrap();
        let link = read_uuid(answer.get().unwrap().get_link().unwrap()).unwrap();

        let mut linked = persons.linked_to_request();
        write_uuid(linked.get().init_id(), Some(a));
        let answer = linked.send().promise.await.unwrap();
        assert_eq!(uuids(answer.get().unwrap().get_ids().unwrap()), [b]);
        let answer = persons.proposals_request().send().promise.await.unwrap();
        assert_eq!(answer.get().unwrap().get_items().unwrap().len(), 0);

        let mut again = persons.confirm_same_request();
        write_uuid(again.get().init_a(), Some(a));
        write_uuid(again.get().init_b(), Some(b));
        expect_problem(again.send().promise.await, CONFLICT);

        let mut unlink = persons.unlink_same_request();
        write_uuid(unlink.get().init_link(), Some(link));
        unlink.send().promise.await.unwrap();
        let mut linked = persons.linked_to_request();
        write_uuid(linked.get().init_id(), Some(a));
        let answer = linked.send().promise.await.unwrap();
        assert!(uuids(answer.get().unwrap().get_ids().unwrap()).is_empty());
    })
    .await;
}

async fn state_of(
    persons: &fs_interfaces::people_capnp::persons::Client,
    a: uuid::Uuid,
    b: uuid::Uuid,
) -> SameState {
    let mut req = persons.same_state_request();
    write_uuid(req.get().init_a(), Some(a));
    write_uuid(req.get().init_b(), Some(b));
    let answer = req.send().promise.await.unwrap();
    answer.get().unwrap().get_state().unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn a_no_is_kept_and_taken_back() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let a = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let b = create(&persons, "uid-2", "A. King", "ada@x.org").await;
        assert!(matches!(state_of(&persons, a, b).await, SameState::Open));

        let mut reject = persons.reject_same_request();
        write_uuid(reject.get().init_a(), Some(b));
        write_uuid(reject.get().init_b(), Some(a));
        let answer = reject.send().promise.await.unwrap();
        let rejection = read_uuid(answer.get().unwrap().get_rejection().unwrap()).unwrap();
        assert!(matches!(
            state_of(&persons, a, b).await,
            SameState::Rejected
        ));
        let answer = persons.proposals_request().send().promise.await.unwrap();
        assert_eq!(
            answer.get().unwrap().get_items().unwrap().len(),
            0,
            "not asked again"
        );

        let mut twice = persons.reject_same_request();
        write_uuid(twice.get().init_a(), Some(a));
        write_uuid(twice.get().init_b(), Some(b));
        expect_problem(twice.send().promise.await, CONFLICT);

        let mut reopen = persons.reopen_same_request();
        write_uuid(reopen.get().init_rejection(), Some(rejection));
        reopen.send().promise.await.unwrap();
        assert!(matches!(state_of(&persons, a, b).await, SameState::Open));
        let answer = persons.proposals_request().send().promise.await.unwrap();
        assert_eq!(answer.get().unwrap().get_items().unwrap().len(), 1);

        let mut confirm = persons.confirm_same_request();
        write_uuid(confirm.get().init_a(), Some(a));
        write_uuid(confirm.get().init_b(), Some(b));
        confirm.send().promise.await.unwrap();
        assert!(matches!(
            state_of(&persons, a, b).await,
            SameState::Confirmed
        ));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn lists_hold_people() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let ada = create(&persons, "uid-1", "Ada", "ada@x.org").await;

        let mut make = persons.create_list_request();
        make.get().set_name("Friends");
        let answer = make.send().promise.await.unwrap();
        let person = answer.get().unwrap().get_person().unwrap();
        let list = id_of(person);
        assert_eq!(json_of(person)["kind"], "group");

        let mut add = persons.add_to_list_request();
        write_uuid(add.get().init_list(), Some(list));
        write_uuid(add.get().init_member(), Some(ada));
        let answer = add.send().promise.await.unwrap();
        let card = json_of(answer.get().unwrap().get_person().unwrap());
        assert_eq!(card["members"]["uid-1"], true);

        let mut members = persons.list_members_request();
        write_uuid(members.get().init_list(), Some(list));
        let answer = members.send().promise.await.unwrap();
        let items = answer.get().unwrap().get_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(id_of(items.get(0)), ada);

        let mut of = persons.lists_of_request();
        write_uuid(of.get().init_member(), Some(ada));
        let answer = of.send().promise.await.unwrap();
        let items = answer.get().unwrap().get_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(id_of(items.get(0)), list);

        let mut remove = persons.remove_from_list_request();
        write_uuid(remove.get().init_list(), Some(list));
        write_uuid(remove.get().init_member(), Some(ada));
        let answer = remove.send().promise.await.unwrap();
        let card = json_of(answer.get().unwrap().get_person().unwrap());
        assert!(card.get("members").is_none());

        let mut from_a_person = persons.remove_from_list_request();
        write_uuid(from_a_person.get().init_list(), Some(ada));
        write_uuid(from_a_person.get().init_member(), Some(list));
        expect_problem(from_a_person.send().promise.await, CONFLICT);
        let mut of_a_person = persons.list_members_request();
        write_uuid(of_a_person.get().init_list(), Some(ada));
        expect_problem(of_a_person.send().promise.await, CONFLICT);

        let mut into_a_person = persons.add_to_list_request();
        write_uuid(into_a_person.get().init_list(), Some(ada));
        write_uuid(into_a_person.get().init_member(), Some(list));
        expect_problem(into_a_person.send().promise.await, CONFLICT);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_deleted_member_is_gone_from_the_list_and_an_untyped_relation_can_be_named() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let ada = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let mut make = persons.create_list_request();
        make.get().set_name("Friends");
        let answer = make.send().promise.await.unwrap();
        let list = id_of(answer.get().unwrap().get_person().unwrap());
        let mut add = persons.add_to_list_request();
        write_uuid(add.get().init_list(), Some(list));
        write_uuid(add.get().init_member(), Some(ada));
        add.send().promise.await.unwrap();
        let mut delete = persons.delete_request();
        write_uuid(delete.get().init_id(), Some(ada));
        delete.send().promise.await.unwrap();
        let mut get = persons.get_request();
        write_uuid(get.get().init_id(), Some(list));
        let answer = get.send().promise.await.unwrap();
        let card = json_of(answer.get().unwrap().get_person().unwrap());
        assert!(card.get("members").is_none(), "{card}");

        let text = r#"{"@type":"Card","version":"1.0","uid":"u","name":{"full":"U"},"relatedTo":{"x":{"@type":"Relation"}}}"#;
        let mut create_it = persons.create_request();
        create_it.get().set_jscontact(text);
        let answer = create_it.send().promise.await.unwrap();
        let id = id_of(answer.get().unwrap().get_person().unwrap());
        let mut relations = persons.relations_request();
        write_uuid(relations.get().init_id(), Some(id));
        let answer = relations.send().promise.await.unwrap();
        let items = answer.get().unwrap().get_items().unwrap();
        assert_eq!(items.get(0).get_uid().unwrap().to_str().unwrap(), "x");
        assert_eq!(items.get(0).get_relation().unwrap().to_str().unwrap(), "");
        let mut unrelate = persons.unrelate_request();
        write_uuid(unrelate.get().init_id(), Some(id));
        unrelate.get().set_other_uid("x");
        let answer = unrelate.send().promise.await.unwrap();
        let card = json_of(answer.get().unwrap().get_person().unwrap());
        assert!(card.get("relatedTo").is_none(), "{card}");
    })
    .await;
}
