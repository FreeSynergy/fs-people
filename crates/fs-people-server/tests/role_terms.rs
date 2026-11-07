//! The SKOS terms of a role definition over a real socket: `type`, `broader`, `closeMatch`,
//! `standardRefs` arrive as defined and as shipped.

mod common;

use common::{BAD_REQUEST, expect_problem, roles_of, with_server};
use fs_interfaces::people_capnp::{RoleScope, role_definition, roles};

const BASE: &str = "https://freesynergy.net/ns/roles/";
const THEIRS: &str = "https://other.example/roles/helper";
const TERM: &str = "https://schema.org/EmployeeRole";

fn texts(list: capnp::text_list::Reader<'_>) -> Vec<String> {
    list.iter()
        .map(|t| t.unwrap().to_string().unwrap())
        .collect()
}

fn base(mut d: role_definition::Builder<'_>, id: &str) {
    d.set_id(id);
    let mut names = d.reborrow().init_names(1);
    names.reborrow().get(0).set_language("en");
    names.reborrow().get(0).set_text("Treasurer");
    d.set_owner("kal");
    d.set_scope(RoleScope::Personal);
}

fn put(mut out: capnp::text_list::Builder<'_>, values: &[&str]) {
    for (i, v) in (0u32..).zip(values) {
        out.set(i, v);
    }
}

fn len(values: &[&str]) -> u32 {
    u32::try_from(values.len()).unwrap()
}

async fn fetch(roles: &roles::Client, id: &str) -> role_definition::Reader<'static> {
    // The answer owns its message; keep it alive for the length of the test by leaking it.
    let mut get = roles.get_request();
    get.get().set_id(id);
    let answer = Box::leak(Box::new(get.send().promise.await.unwrap()));
    let r = answer.get().unwrap();
    assert!(r.get_found(), "{id} is known");
    r.get_definition().unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn a_defined_role_keeps_all_its_terms() {
    with_server(|root| async move {
        let roles = roles_of(&root).await;
        let mut define = roles.define_request();
        let mut d = define.get().init_definition();
        base(d.reborrow(), "treasurer");
        d.set_type("org:Role");
        let parent = format!("{BASE}member");
        let v = [parent.as_str()];
        put(d.reborrow().init_broader(len(&v)), &v);
        let v = [THEIRS];
        put(d.reborrow().init_close_match(len(&v)), &v);
        let v = [TERM];
        put(d.reborrow().init_standard_refs(len(&v)), &v);
        define.send().promise.await.unwrap();

        let d = fetch(&roles, "treasurer").await;
        assert_eq!(
            d.get_id().unwrap().to_str().unwrap(),
            format!("{BASE}treasurer")
        );
        assert_eq!(d.get_type().unwrap().to_str().unwrap(), "org:Role");
        assert_eq!(texts(d.get_broader().unwrap()), [format!("{BASE}member")]);
        assert_eq!(texts(d.get_close_match().unwrap()), [THEIRS]);
        assert_eq!(texts(d.get_standard_refs().unwrap()), [TERM]);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_empty_type_is_a_concept_and_no_terms_stay_empty() {
    with_server(|root| async move {
        let roles = roles_of(&root).await;
        let mut define = roles.define_request();
        base(define.get().init_definition(), "friend");
        define.send().promise.await.unwrap();

        let d = fetch(&roles, "friend").await;
        assert_eq!(d.get_type().unwrap().to_str().unwrap(), "skos:Concept");
        assert!(d.get_broader().unwrap().is_empty());
        assert!(d.get_close_match().unwrap().is_empty());
        assert!(d.get_standard_refs().unwrap().is_empty());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn shipped_roles_say_what_kind_they_are() {
    with_server(|root| async move {
        let roles = roles_of(&root).await;
        for (id, kind) in [
            ("member", "org:Role"),
            ("user", "org:Role"),
            ("customer", "skos:Concept"),
            ("supplier", "skos:Concept"),
            ("lead", "skos:Concept"),
            ("prospect", "skos:Concept"),
        ] {
            let d = fetch(&roles, id).await;
            assert_eq!(d.get_type().unwrap().to_str().unwrap(), kind, "{id}");
        }
        // The full id finds the same role as the short one.
        let d = fetch(&roles, &format!("{BASE}customer")).await;
        assert_eq!(
            d.get_id().unwrap().to_str().unwrap(),
            format!("{BASE}customer")
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_type_that_is_no_role_type_and_a_second_parent_are_refused() {
    with_server(|root| async move {
        let roles = roles_of(&root).await;
        let mut odd = roles.define_request();
        let mut d = odd.get().init_definition();
        base(d.reborrow(), "odd");
        d.set_type("foaf:Person");
        expect_problem(odd.send().promise.await, BAD_REQUEST);

        let mut two = roles.define_request();
        let mut d = two.get().init_definition();
        base(d.reborrow(), "two");
        let parent = format!("{BASE}member");
        let v = [THEIRS, parent.as_str()];
        put(d.reborrow().init_broader(len(&v)), &v);
        expect_problem(two.send().promise.await, BAD_REQUEST);
    })
    .await;
}
