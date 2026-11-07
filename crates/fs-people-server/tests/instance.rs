//! The support report names the store of the provider (`SupportReport.instance`), and only there.

mod common;

use std::sync::Arc;

use common::{ME, create, persons_of, with_root, with_store};
use fs_interfaces::people_capnp::people;
use fs_interfaces::rpc::read_uuid;
use fs_objects::{ActivityFilter, Store, TursoStore};
use fs_people_core::PeopleRepo;
use fs_people_server::PeopleServer;
use uuid::Uuid;

async fn instance_of(root: &people::Client) -> Option<Uuid> {
    let answer = root.support_report_request().send().promise.await.unwrap();
    let report = answer.get().unwrap().get_report().unwrap();
    read_uuid(report.get_instance().unwrap())
}

#[tokio::test(flavor = "current_thread")]
async fn the_report_carries_the_id_of_the_store_and_the_log_does_not() {
    with_store(|root, store| async move {
        let told = instance_of(&root)
            .await
            .expect("the report names an instance");
        assert_eq!(told, store.identity().as_uuid());
        assert_eq!(instance_of(&root).await, Some(told));

        let persons = persons_of(&root).await;
        create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let log = store.activities(&ActivityFilter::default()).await.unwrap();
        assert!(!log.is_empty());
        let written = format!("{log:?}");
        assert!(!written.contains(&told.to_string()), "{written}");
    })
    .await;
}

async fn serve_file(path: &std::path::Path) -> (Uuid, Uuid) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open(path).await.unwrap());
    let repo = Arc::new(PeopleRepo::new(store.clone(), ME));
    let root = PeopleServer::new(repo.clone(), repo.clone(), repo).into_client();
    let told = std::rc::Rc::new(std::cell::Cell::new(None));
    let slot = told.clone();
    with_root(root, |root| async move {
        slot.set(instance_of(&root).await);
    })
    .await;
    (told.get().unwrap(), store.identity().as_uuid())
}

#[tokio::test(flavor = "current_thread")]
async fn the_id_stays_over_a_restart_and_a_new_store_has_another() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let (first, stored) = serve_file(&dir.path().join("a.db")).await;
            assert_eq!(first, stored);
            let (again, _) = serve_file(&dir.path().join("a.db")).await;
            assert_eq!(again, first);
            let (other, _) = serve_file(&dir.path().join("b.db")).await;
            assert_ne!(other, first);
        })
        .await;
}
