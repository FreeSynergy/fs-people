//! Roles through the trait `RoleBook`, and the explicit "not possible" of everything else.
//!
//! Integration tests against the trait; no stand-ins.

mod common;

use common::{Served, add, person};
use fs_people_core::{
    AI_ROLE_IRI, Card, Error, People, RoleBook, RoleDefinition, RoleKind, RoleScope, is_ai_role,
};

#[tokio::test(flavor = "multi_thread")]
async fn a_role_is_read_with_its_definition_or_is_not_there() {
    let served = Served::start().await;
    let book = served.client();
    let mut own = RoleDefinition::new("friend", "Friend", "fs-people", RoleScope::Personal);
    own.description.insert("de".into(), "Freund".into());
    served.repo.define_role(own.clone()).await.unwrap();
    let got = book.role("friend").await.unwrap().unwrap();
    assert_eq!(got.names, own.names);
    assert_eq!(got.description, own.description);
    assert_eq!(
        (got.owner.as_str(), got.scope),
        ("fs-people", RoleScope::Personal)
    );
    assert!(book.role("nobody-knows").await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_skos_terms_of_a_role_are_read_not_lost() {
    let served = Served::start().await;
    let book = served.client();
    let mut own = RoleDefinition::new("mentor", "Mentor", "fs-people", RoleScope::Personal);
    own.kind = RoleKind::OrgRole;
    own.broader = Some("https://example.org/roles/guide".into());
    own.close_match = Some("https://schema.org/Role".into());
    own.standard_refs = vec!["https://www.w3.org/ns/org#Role".into()];
    served.repo.define_role(own.clone()).await.unwrap();
    let got = book.role("mentor").await.unwrap().unwrap();
    assert_eq!(got.id, own.id, "the full id");
    assert_eq!(got.kind, RoleKind::OrgRole);
    assert_eq!(got.broader, own.broader);
    assert_eq!(got.close_match, own.close_match);
    assert_eq!(got.standard_refs, own.standard_refs);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_roles_of_a_person_come_with_their_assignment() {
    let served = Served::start().await;
    let people = served.client();
    let bot = add(&people, Card::application("Bot")).await;
    let ada = add(&people, person("Ada", "a@x.org")).await;
    served.repo.mark_ai(bot).await.unwrap();
    let held = people.roles_of(bot).await.unwrap();
    assert_eq!(held.len(), 1);
    assert_eq!((held[0].role.as_str(), held[0].person), (AI_ROLE_IRI, bot));
    assert!(is_ai_role(&held[0].role));
    assert_eq!(held[0].scope, None);
    assert!(people.roles_of(ada).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn what_the_client_does_not_offer_is_unsupported_in_so_many_words() {
    let served = Served::start().await;
    let people = served.client();
    let id = add(&people, person("Ada", "a@x.org")).await;
    let unsupported = |e: Error| matches!(e, Error::Unsupported(_));
    assert!(unsupported(people.get_by_uid("x").await.unwrap_err()));
    assert!(unsupported(
        people.find_by_email("a@x.org").await.unwrap_err()
    ));
    assert!(unsupported(people.create_group("G").await.unwrap_err()));
    assert!(unsupported(people.relations(id).await.unwrap_err()));
    assert!(unsupported(
        people.find_same_person_proposals().await.unwrap_err()
    ));
    assert!(unsupported(people.export_vcard(id).await.unwrap_err()));
    assert!(unsupported(people.mark_ai(id).await.unwrap_err()));
    assert!(unsupported(people.roles().await.unwrap_err()));
    let own = RoleDefinition::new("x", "X", "fs-people", RoleScope::Personal);
    assert!(unsupported(people.define_role(own).await.unwrap_err()));
}
