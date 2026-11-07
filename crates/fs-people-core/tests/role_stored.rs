//! Older stored configurations stay readable, the old array merges, ids stay sane, loads are atomic.

mod common;

use std::sync::Arc;

use common::{ME, repo, setup};
use fs_people_core::{
    AI_ROLE_IRI, CONTEXT, Error, PeopleRepo, RoleDefinition, RoleDirectory, RoleRegistry,
    RoleScope, full_id,
};
use fs_objects::{Envelope, Store, StoredObject};
use serde_json::{Value, json};

const OURS: &str = "https://freesynergy.net/ns/roles/";

fn file(base: &str, id: &str) -> String {
    json!({
        "@context": [{"@base": base}], "id": "", "type": "skos:ConceptScheme",
        "roles": [{"id": id, "names": {"en": id}, "owner": "kal", "scope": "personal"}]
    })
    .to_string()
}

async fn put_stored(store: &Arc<dyn Store>, content: Value) {
    let mut env = Envelope::new("RoleConfig", "older");
    env.context = Some(CONTEXT.into());
    let obj = StoredObject::new(env, "application/json", content);
    store.put(obj, ME).await.unwrap();
}

#[tokio::test]
async fn an_older_configuration_with_a_sentence_as_owner_is_read_and_migrated() {
    let (r, store) = setup().await;
    let old = json!([{"id": "coach", "names": {"en": "Coach"}, "owner": "CRM (later)", "scope": "group"}]);
    put_stored(&store, old).await;
    let again = PeopleRepo::new(store, ME);
    assert!(
        again
            .restore_role_config()
            .await
            .unwrap()
            .skipped()
            .is_empty()
    );
    let coach = again.role("coach").await.unwrap().unwrap();
    assert_eq!(coach.owner, "crm-later");
    // Newly typed input stays strict.
    let strict = json!([{"id": "x", "names": {"en": "X"}, "owner": "CRM (later)"}]);
    assert!(matches!(
        r.store_role_config(&strict.to_string()).await,
        Err(Error::Invalid(_))
    ));
}

#[tokio::test]
async fn a_broken_stored_object_blocks_neither_the_start_nor_a_new_save() {
    let (_, store) = setup().await;
    put_stored(&store, json!({"nonsense": true})).await;
    let r = PeopleRepo::new(store, ME);
    let skipped = r.restore_role_config().await.unwrap().skipped().to_vec();
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    r.store_role_config(&file("https://example.org/r/", "a"))
        .await
        .unwrap();
    assert!(r.role("https://example.org/r/a").await.unwrap().is_some());
    assert!(r.role("member").await.unwrap().is_some());
}

#[tokio::test]
async fn the_old_array_merges_over_the_shipped_roles() {
    let dir = RoleRegistry::known();
    let array = json!([
        {"id": "coach", "names": {"en": "Coach"}, "owner": "kal"},
        {"id": "member", "names": {"en": "Mate"}, "owner": "kal", "scope": "group"}
    ]);
    dir.load(&array.to_string()).unwrap();
    assert!(dir.find("ai").is_some() && dir.find("customer").is_some());
    assert!(dir.find("coach").is_some());
    assert_eq!(dir.find("member").unwrap().names["en"], "Mate");
    // A stored array merges on restore as well.
    let (_, store) = setup().await;
    put_stored(&store, array).await;
    let r = PeopleRepo::new(store, ME);
    assert!(r.restore_role_config().await.unwrap().skipped().is_empty());
    assert!(r.role(AI_ROLE_IRI).await.unwrap().is_some());
}

#[test]
fn an_array_cannot_bring_foreign_full_ids() {
    let dir = RoleRegistry::known();
    let foreign = json!([{"id": "https://example.org/r/ai", "names": {"en": "x"}, "owner": "kal"}]);
    assert!(matches!(
        dir.load(&foreign.to_string()),
        Err(Error::Invalid(_))
    ));
    assert_eq!(dir.find("ai").unwrap().id, AI_ROLE_IRI);
}

#[test]
fn a_short_id_has_no_slash_and_is_not_empty() {
    let dir = RoleRegistry::empty();
    for bad in ["a/b", "a#b", "", "  "] {
        let loaded = dir.load(&file("https://example.org/r/", bad));
        assert!(matches!(loaded, Err(Error::Invalid(_))), "{bad:?}");
    }
    let array = json!([{"id": "", "names": {"en": "x"}, "owner": "kal"}]);
    assert!(matches!(
        dir.load(&array.to_string()),
        Err(Error::Invalid(_))
    ));
    assert!(dir.all().is_empty());
}

#[tokio::test]
async fn define_role_refuses_an_empty_id() {
    let r = repo().await;
    for id in ["", " "] {
        let def = RoleDefinition::new(id, "X", "kal", RoleScope::Personal);
        assert!(
            matches!(r.define_role(def).await, Err(Error::Invalid(_))),
            "{id:?}"
        );
    }
    assert!(r.role("").await.unwrap().is_none());
    assert_eq!(full_id("a"), format!("{OURS}a"));
}

#[test]
fn parallel_loads_of_different_files_all_arrive() {
    for round in 0..300 {
        let dir = RoleRegistry::known();
        let handles: Vec<_> = (0..4)
            .map(|n| {
                let dir = dir.clone();
                std::thread::spawn(move || {
                    dir.load(&file(&format!("https://example.org/{n}/"), "a"))
                        .unwrap();
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(dir.bases().len(), 5, "round {round}");
    }
}

/// A decorator that counts questions; the repository takes any `RoleDirectory`.
struct Counting(RoleRegistry, Arc<std::sync::atomic::AtomicUsize>);

impl RoleDirectory for Counting {
    fn preview(
        &self,
        json: &str,
    ) -> fs_people_core::Result<std::collections::BTreeMap<String, RoleDefinition>> {
        self.0.preview(json)
    }
    fn load(&self, json: &str) -> fs_people_core::Result<()> {
        self.0.load(json)
    }
    fn load_stored(&self, json: &str) -> fs_people_core::Result<()> {
        self.0.load_stored(json)
    }
    fn find(&self, id: &str) -> Option<RoleDefinition> {
        self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.0.find(id)
    }
    fn all(&self) -> Vec<RoleDefinition> {
        self.0.all()
    }
    fn bases(&self) -> Vec<String> {
        self.0.bases()
    }
}

#[tokio::test]
async fn the_repository_asks_whatever_directory_it_is_given() {
    let asked = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let r = repo()
        .await
        .with_roles(Counting(RoleRegistry::known(), asked.clone()));
    assert!(r.role("member").await.unwrap().is_some());
    assert!(asked.load(std::sync::atomic::Ordering::SeqCst) > 0);
}
