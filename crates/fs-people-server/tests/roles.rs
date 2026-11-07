//! The `Roles` interface over a real socket, against the store-backed provider.

mod common;

use common::{
    CLUB, CONFLICT, INVALID, create, expect_problem, persons_of, roles_of, uuids, with_server,
};
use fs_interfaces::people_capnp::{RoleScope, roles};
use fs_interfaces::rpc::{read_uuid, write_uuid};
use serde_json::json;

fn fill_coach(mut d: fs_interfaces::people_capnp::role_definition::Builder<'_>) {
    d.set_id("coach");
    let mut names = d.reborrow().init_names(2);
    names.reborrow().get(0).set_language("en");
    names.reborrow().get(0).set_text("Coach");
    names.reborrow().get(1).set_language("de");
    names.reborrow().get(1).set_text("Trainer");
    let mut about = d.reborrow().init_description(1);
    about.reborrow().get(0).set_language("en");
    about.reborrow().get(0).set_text("Trains a team");
    d.set_standard("org:Role");
    let mut fields = d.reborrow().init_fields(1);
    let mut field = fields.reborrow().get(0);
    field.set_name("licence");
    field.set_standard("none yet");
    let mut text = field.init_description(1);
    text.reborrow().get(0).set_language("en");
    text.reborrow().get(0).set_text("Licence number");
    d.set_owner("fs-people");
    d.set_scope(RoleScope::Group);
    d.set_card_kind("individual");
}

const BASE: &str = "https://freesynergy.net/ns/roles/";

fn iris(short: &[&str]) -> Vec<String> {
    short.iter().map(|id| format!("{BASE}{id}")).collect()
}

async fn all_ids(roles: &roles::Client) -> Vec<String> {
    let answer = roles.list_request().send().promise.await.unwrap();
    let items = answer.get().unwrap().get_items().unwrap();
    items
        .iter()
        .map(|d| d.get_id().unwrap().to_string().unwrap())
        .collect()
}

#[tokio::test(flavor = "current_thread")]
async fn definitions_arrive_as_they_were_defined() {
    with_server(|root| async move {
        let roles = roles_of(&root).await;
        assert_eq!(
            all_ids(&roles).await,
            iris(&[
                "ai", "customer", "lead", "member", "prospect", "supplier", "user"
            ])
        );

        let mut define = roles.define_request();
        fill_coach(define.get().init_definition());
        define.send().promise.await.unwrap();
        assert_eq!(
            all_ids(&roles).await,
            iris(&[
                "ai", "coach", "customer", "lead", "member", "prospect", "supplier", "user"
            ])
        );

        let mut get = roles.get_request();
        get.get().set_id("coach");
        let answer = get.send().promise.await.unwrap();
        let r = answer.get().unwrap();
        assert!(r.get_found());
        let d = r.get_definition().unwrap();
        assert_eq!(d.get_names().unwrap().len(), 2);
        assert_eq!(
            d.get_names()
                .unwrap()
                .get(0)
                .get_language()
                .unwrap()
                .to_str()
                .unwrap(),
            "de"
        );
        assert_eq!(d.get_description().unwrap().len(), 1);
        assert_eq!(d.get_standard().unwrap().to_str().unwrap(), "org:Role");
        let field = d.get_fields().unwrap().get(0);
        assert_eq!(field.get_name().unwrap().to_str().unwrap(), "licence");
        assert_eq!(field.get_standard().unwrap().to_str().unwrap(), "none yet");
        assert_eq!(field.get_description().unwrap().len(), 1);
        assert_eq!(d.get_owner().unwrap().to_str().unwrap(), "fs-people");
        assert_eq!(d.get_scope().unwrap(), RoleScope::Group);
        assert_eq!(d.get_card_kind().unwrap().to_str().unwrap(), "individual");

        let mut twice = roles.define_request();
        fill_coach(twice.get().init_definition());
        expect_problem(twice.send().promise.await, CONFLICT);
        let empty = roles.define_request();
        expect_problem(empty.send().promise.await, INVALID);

        let mut undefine = roles.undefine_request();
        undefine.get().set_id("coach");
        undefine.send().promise.await.unwrap();
        let mut get = roles.get_request();
        get.get().set_id("coach");
        let answer = get.send().promise.await.unwrap();
        assert!(!answer.get().unwrap().get_found());

        let mut shipped = roles.undefine_request();
        shipped.get().set_id("member");
        expect_problem(shipped.send().promise.await, CONFLICT);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_configuration_is_checked_then_added_or_swapped() {
    with_server(|root| async move {
        let roles = roles_of(&root).await;
        let mut bad = roles.load_config_request();
        bad.get()
            .set_json("[{\"id\":\"x\",\"names\":{},\"owner\":\"o\"}]");
        expect_problem(bad.send().promise.await, INVALID);
        let mut broken = roles.load_config_request();
        broken.get().set_json("no json");
        expect_problem(broken.send().promise.await, INVALID);
        assert_eq!(all_ids(&roles).await.len(), 7, "nothing changed");

        let good = json!([{"id": "x", "names": {"en": "X"}, "owner": "o", "scope": "public"}]);
        let mut load = roles.load_config_request();
        load.get().set_json(good.to_string().as_str());
        load.send().promise.await.unwrap();
        let mut all = iris(&["ai", "customer", "lead", "member", "prospect", "supplier"]);
        all.extend(iris(&["user", "x"]));
        assert_eq!(
            all_ids(&roles).await,
            all,
            "the array form is added to the shipped roles"
        );

        // A scheme file replaces the scheme of its base: only what it lists stays.
        let scheme = json!({
            "@context": [{"@base": BASE}], "id": "", "type": "skos:ConceptScheme",
            "names": {"en": "Mine"},
            "roles": [{"id": "y", "type": "skos:Concept", "names": {"en": "Y"},
                       "owner": "o", "scope": "public"}]
        });
        let mut load = roles.load_config_request();
        load.get().set_json(scheme.to_string().as_str());
        load.send().promise.await.unwrap();
        assert_eq!(all_ids(&roles).await, iris(&["y"]));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn assignments_come_and_go() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let ada = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let bob = create(&persons, "uid-2", "Bob", "bob@x.org").await;

        let mut assign = roles.assign_request();
        write_uuid(assign.get().init_person(), Some(ada));
        assign.get().set_role("member");
        assign.get().set_scope(CLUB);
        let answer = assign.send().promise.await.unwrap();
        let held = answer.get().unwrap().get_assignment().unwrap();
        let id = read_uuid(held.get_id().unwrap()).unwrap();
        assert_eq!(read_uuid(held.get_person().unwrap()), Some(ada));
        assert_eq!(held.get_scope().unwrap().to_str().unwrap(), CLUB);

        let mut elsewhere = roles.assign_request();
        write_uuid(elsewhere.get().init_person(), Some(bob));
        elsewhere.get().set_role("member");
        elsewhere
            .get()
            .set_scope("https://example.org/groups/other");
        elsewhere.send().promise.await.unwrap();

        let mut of = roles.roles_of_request();
        write_uuid(of.get().init_person(), Some(ada));
        let answer = of.send().promise.await.unwrap();
        let items = answer.get().unwrap().get_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(
            items.get(0).get_role().unwrap().to_str().unwrap(),
            format!("{BASE}member"),
            "the answer carries the full id; the short one was accepted in the request"
        );
        assert_eq!(items.get(0).get_details().unwrap().to_str().unwrap(), "{}");
        assert_eq!(read_uuid(items.get(0).get_id().unwrap()), Some(id));
        assert_eq!(read_uuid(items.get(0).get_person().unwrap()), Some(ada));
        assert_eq!(items.get(0).get_scope().unwrap().to_str().unwrap(), CLUB);

        // Empty is personal, and these two hold it within groups.
        let mut with = roles.people_with_request();
        with.get().set_role("member");
        let answer = with.send().promise.await.unwrap();
        assert!(uuids(answer.get().unwrap().get_ids().unwrap()).is_empty());
        let mut literal = roles.people_with_request();
        literal.get().set_role("member");
        literal.get().set_scope("personal");
        expect_problem(literal.send().promise.await, INVALID);
        let mut with = roles.people_with_request();
        with.get().set_role("member");
        with.get().set_scope(CLUB);
        let answer = with.send().promise.await.unwrap();
        assert_eq!(uuids(answer.get().unwrap().get_ids().unwrap()), [ada]);

        let mut unassign = roles.unassign_request();
        write_uuid(unassign.get().init_assignment(), Some(id));
        unassign.send().promise.await.unwrap();
        let mut with = roles.people_with_request();
        with.get().set_role("member");
        with.get().set_scope(CLUB);
        let answer = with.send().promise.await.unwrap();
        assert!(uuids(answer.get().unwrap().get_ids().unwrap()).is_empty());
    })
    .await;
}
