//! Short ids and full ids of roles side by side: an AI stays an AI, older data is read, files stay apart.

mod common;

use common::{ME, repo, setup};
use fs_people_core::{
    AI_ROLE, AI_ROLE_IRI, CONTEXT, Card, Error, PeopleRepo, RELATIONSHIP_TYPE, RoleDefinition,
    RoleScope, full_id,
};
use fs_store::{Envelope, ObjectId, StoredObject};
use serde_json::{Map, json};

const MINE: &str = "https://example.org/roles/kal/";
const CLUB: &str = "https://local.invalid/groups/club";

/// An assignment as older versions stored it: the short id of the role in the index.
async fn put_old_assignment(store: &dyn fs_store::Store, person: ObjectId, role: &str) {
    let mut env = Envelope::new(RELATIONSHIP_TYPE, "older");
    env.context = Some(CONTEXT.into());
    let content = json!({"type": RELATIONSHIP_TYPE, "relationship": role, "details": {}});
    let obj = StoredObject::new(env, "application/activity+json", content)
        .with_index("role", role)
        .with_index("person", person.to_string())
        .with_index("scope", "personal");
    store.put(obj, ME).await.unwrap();
}

#[tokio::test]
async fn an_ai_is_an_ai_whichever_form_its_role_was_stored_in() {
    let (r, store) = setup().await;
    let old = r
        .create(Card::application("Old"))
        .await
        .unwrap()
        .envelope
        .id;
    put_old_assignment(&*store, old, AI_ROLE).await;
    let new = r
        .create(Card::application("New"))
        .await
        .unwrap()
        .envelope
        .id;
    r.mark_ai(new).await.unwrap();

    assert!(
        r.is_ai(old).await.unwrap(),
        "short id stored by an older version"
    );
    assert!(r.is_ai(new).await.unwrap());
    assert_eq!(r.roles_of(old).await.unwrap()[0].role, AI_ROLE_IRI);
    let mut both = [old, new];
    both.sort_unstable();
    for form in [AI_ROLE, AI_ROLE_IRI] {
        assert_eq!(
            r.people_with_role(form, None).await.unwrap(),
            both,
            "{form}"
        );
        assert_eq!(
            r.people_with_personal_role(form).await.unwrap(),
            both,
            "{form}"
        );
    }
    // Marking an old AI again holds one assignment, not two.
    r.mark_ai(old).await.unwrap();
    assert_eq!(r.roles_of(old).await.unwrap().len(), 1);
}

#[test]
fn consumers_recognise_the_ai_role_in_both_forms() {
    use fs_people_core::is_ai_role;
    assert!(is_ai_role("ai") && is_ai_role(AI_ROLE_IRI));
    assert!(!is_ai_role("customer") && !is_ai_role(&format!("{MINE}ai")));
}

#[tokio::test]
async fn the_ai_role_with_a_wrong_kind_stays_refused_by_full_id() {
    let r = repo().await;
    let human = r.create(Card::individual("Ada")).await.unwrap().envelope.id;
    let by_full = r.assign_role(human, AI_ROLE_IRI, None, Map::new()).await;
    assert!(matches!(by_full, Err(Error::Conflict(_))));
    assert!(!r.is_ai(human).await.unwrap());
}

#[tokio::test]
async fn customers_and_suppliers_may_be_organisations() {
    let r = repo().await;
    let firm = r
        .create(Card::new("org", "Acme"))
        .await
        .unwrap()
        .envelope
        .id;
    for role in ["customer", "supplier", "lead", "prospect"] {
        r.assign_role(firm, role, Some(CLUB), Map::new())
            .await
            .unwrap();
    }
    let held = r.roles_of(firm).await.unwrap();
    assert_eq!(held.len(), 4);
    assert_eq!(
        r.people_with_role(&full_id("lead"), Some(CLUB))
            .await
            .unwrap(),
        [firm]
    );
}

#[tokio::test]
async fn lead_prospect_and_customer_are_three_roles_with_no_stage() {
    let r = repo().await;
    let ada = r.create(Card::individual("Ada")).await.unwrap().envelope.id;
    r.assign_role(ada, "lead", Some(CLUB), Map::new())
        .await
        .unwrap();
    assert!(
        r.people_with_role("prospect", None)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        r.people_with_role("customer", None)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn my_customer_and_our_customer_are_two_roles_and_files_survive_a_restart() {
    let (r, store) = setup().await;
    let file = json!({
        "@context": ["https://freesynergy.net/ns/roles.jsonld", {"@base": MINE}],
        "id": "", "type": "skos:ConceptScheme", "names": {"de": "Meine Rollen"},
        "roles": [{"id": "customer", "names": {"de": "Stammkunde"}, "owner": "kal", "scope": "group"}]
    });
    r.store_role_config(&file.to_string()).await.unwrap();
    let ada = r.create(Card::individual("Ada")).await.unwrap().envelope.id;
    r.assign_role(ada, "customer", Some(CLUB), Map::new())
        .await
        .unwrap();
    r.assign_role(ada, &format!("{MINE}customer"), Some(CLUB), Map::new())
        .await
        .unwrap();
    assert_eq!(r.roles_of(ada).await.unwrap().len(), 2);
    assert!(r.role("member").await.unwrap().is_some(), "ours stays");

    let again = PeopleRepo::new(store, ME);
    assert!(
        again
            .role(&format!("{MINE}customer"))
            .await
            .unwrap()
            .is_none()
    );
    again.restore_role_config().await.unwrap();
    assert!(
        again
            .role(&format!("{MINE}customer"))
            .await
            .unwrap()
            .is_some()
    );
    assert!(again.role("member").await.unwrap().is_some());
    // The file in use cannot be dropped from under a holder.
    let empty = file.to_string().replace("\"customer\"", "\"other\"");
    assert!(matches!(
        again.store_role_config(&empty).await,
        Err(Error::Conflict(_))
    ));
}

#[tokio::test]
async fn own_roles_by_define_role_are_found_by_both_forms_and_unknown_stays_unknown() {
    let r = repo().await;
    r.define_role(RoleDefinition::new(
        "coach",
        "Coach",
        "kal",
        RoleScope::Group,
    ))
    .await
    .unwrap();
    assert_eq!(
        r.role("coach").await.unwrap(),
        r.role(&full_id("coach")).await.unwrap()
    );
    r.undefine_role("coach").await.unwrap();
    assert!(matches!(
        r.undefine_role("coach").await,
        Err(Error::UnknownRole(_))
    ));
}
