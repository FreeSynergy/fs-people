//! A delete that stops halfway, and what it must not leave behind; lists that do not read; large imports.

mod common;

use common::{ME, person, setup};
use fs_people_core::{Card, Error, PeopleRepo, RoleDefinition, RoleRegistry, RoleScope};
use fs_store::{Envelope, StoredObject};
use serde_json::{Map, json};

const CLUB: &str = "https://x/groups/club";
const V7: &str = "01a1aaef-ba80-7f7a-8dff-0000000000ab";

#[tokio::test]
async fn a_new_card_of_an_id_does_not_inherit_what_a_delete_left() {
    let (r, store) = setup().await;
    let mut card = person("Ada", "a@x.org");
    card.uid = V7.into();
    let id = r.create(card.clone()).await.unwrap().envelope.id;
    r.assign_role(id, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    // A delete that stopped after the card.
    store.delete(id, ME).await.unwrap();

    let said = r.create(card.clone()).await.unwrap_err();
    assert!(matches!(said, Error::Conflict(_)), "{said}");
    let report = r
        .import_jscontact(&serde_json::to_string(&card).unwrap())
        .await
        .unwrap();
    assert_eq!(report.created.len(), 0);
    assert_eq!(report.failed.len(), 1, "{report:?}");

    r.delete(id).await.unwrap();
    r.create(card).await.unwrap();
    assert!(r.roles_of(id).await.unwrap().is_empty());
}

fn broken_list(members: serde_json::Value) -> StoredObject {
    let mut envelope = Envelope::new("Profile", "Broken");
    envelope.context = Some("people".into());
    let content = json!({"uid": 5, "kind": "group", "members": members});
    StoredObject::new(envelope, "application/jscontact+json", content)
        .with_index("kind", "group")
        .with_index("name_sort", "broken")
}

#[tokio::test]
async fn a_list_that_does_not_read_blocks_no_delete_and_no_proposals() {
    let (r, store) = setup().await;
    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let twin = r
        .create(person("A. K.", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let uid = r.get_card(ada).await.unwrap().unwrap().uid;
    let holds = broken_list(json!({uid.clone(): true, "other": true}));
    let holds_id = holds.envelope.id;
    store.put(holds, ME).await.unwrap();
    let unreadable = broken_list(json!(["no", "map"]));
    let other_id = unreadable.envelope.id;
    store.put(unreadable, ME).await.unwrap();

    let proposals = r.find_same_person_proposals().await.unwrap();
    assert_eq!(proposals.len(), 1, "{proposals:?}");
    let bad: Vec<_> = r
        .unreadable_cards()
        .await
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(bad.len(), 2);
    assert!(bad.contains(&holds_id) && bad.contains(&other_id));

    r.delete(ada).await.unwrap();
    let left = store.get(holds_id).await.unwrap().unwrap();
    assert_eq!(left.content["members"], json!({"other": true}));
    assert_eq!(left.index["member"], ["other"]);
    assert!(r.get(twin).await.unwrap().is_some());
}

#[tokio::test]
async fn deleting_a_group_card_ends_the_roles_held_within_it() {
    let (r, _) = setup().await;
    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let group = r.create_group("Club").await.unwrap();
    let iri = group.envelope.iri.clone().unwrap();
    r.assign_role(ada, "member", Some(&iri), Map::new())
        .await
        .unwrap();
    r.assign_role(ada, "member", Some(CLUB), Map::new())
        .await
        .unwrap();
    r.delete(group.envelope.id).await.unwrap();
    let left = r.roles_of(ada).await.unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].scope.as_deref(), Some(CLUB));
}

#[tokio::test]
async fn lists_are_found_by_a_member_in_any_spelling() {
    let (r, store) = setup().await;
    let mut ada = Card::individual("Ada");
    ada.uid = format!("urn:uuid:{}", V7.to_uppercase());
    let id = r.create(ada).await.unwrap().envelope.id;
    let list = r.create_group("New").await.unwrap().envelope.id;
    r.add_member(list, id).await.unwrap();
    let stored = store.get(list).await.unwrap().unwrap();
    assert_eq!(stored.index["member"], [V7], "the index holds one spelling");
    let mut older = stored.clone();
    older.index.insert(
        "member".into(),
        vec![format!("urn:uuid:{}", V7.to_uppercase())],
    );
    store.put(older, ME).await.unwrap();
    let found = r.groups_of(id).await.unwrap();
    assert_eq!(
        found.iter().map(|o| o.envelope.id).collect::<Vec<_>>(),
        [list]
    );
}

#[tokio::test]
async fn role_rules_after_the_second_review() {
    let (r, store) = setup().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;

    // The role of an AI asks another kind: the card keeps its own.
    let wrong = json!([{"id": "ai", "names": {"en": "AI"}, "owner": "o", "scope": "public", "cardKind": "org"}]);
    r.swap_role_config(&wrong.to_string()).unwrap();
    assert!(matches!(r.mark_ai(a).await, Err(Error::Conflict(_))));
    assert_eq!(r.get_card(a).await.unwrap().unwrap().kind(), "individual");

    // Marked under a group-scoped role, the same configuration still loads.
    let group = json!([{"id": "ai", "names": {"en": "AI"}, "owner": "o", "scope": "group", "cardKind": "application"}]);
    r.swap_role_config(&group.to_string()).unwrap();
    r.mark_ai(a).await.unwrap();
    r.store_role_config(&group.to_string()).await.unwrap();

    // An assignment from before a rule blocks no configuration.
    let older = PeopleRepo::new(store, ME).with_roles(RoleRegistry::empty());
    older
        .swap_role_config(
            &json!([{"id": "m", "names": {"en": "M"}, "owner": "o", "scope": "personal"}])
                .to_string(),
        )
        .unwrap();
    let b = older
        .create(person("Bob", "b@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    older.assign_role(b, "m", None, Map::new()).await.unwrap();
    let group_now = json!([{"id": "m", "names": {"en": "M"}, "owner": "o", "scope": "group"},
        {"id": "ai", "names": {"en": "AI"}, "owner": "o", "scope": "group", "cardKind": "application"}]);
    older.swap_role_config(&group_now.to_string()).unwrap();
    older
        .store_role_config(&group_now.to_string())
        .await
        .unwrap();

    let no_kind = json!([{"id": "x", "names": {"en": "X"}, "owner": "o", "cardKind": ""}]);
    assert!(matches!(
        RoleRegistry::check(&no_kind.to_string()),
        Err(Error::Invalid(_))
    ));
    let mut own = RoleDefinition::new("y", "Y", "o", RoleScope::Personal);
    own.card_kind = Some(String::new());
    assert!(matches!(r.define_role(own).await, Err(Error::Invalid(_))));
}
