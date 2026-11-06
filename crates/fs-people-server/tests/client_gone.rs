//! A write goes to its end, also if the client goes away during it; and a half-done delete says so.

#[path = "common/backend.rs"]
mod backend;
mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use backend::Backend;
use common::{ME, card, create, persons_of, roles_of, uuids};
use fs_interfaces::people_capnp::people;
use fs_interfaces::rpc::{Problem, bind_socket, connect, serve, write_uuid};
use fs_people_core::PeopleRepo;
use fs_people_server::PeopleServer;
use fs_store::{Store, TursoStore};
use uuid::Uuid;

const PAUSE: Duration = Duration::from_millis(3);

async fn with_socket<F, Fut>(pause: Duration, test: F)
where
    F: FnOnce(std::path::PathBuf, Arc<Backend>) -> Fut,
    Fut: Future<Output = ()>,
{
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("people.sock");
            let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
            let backend = Arc::new(Backend {
                store,
                pause,
                fail_others: AtomicBool::new(false),
            });
            let repo = Arc::new(PeopleRepo::with_backend(
                backend.clone(),
                backend.clone(),
                ME,
            ));
            let root = PeopleServer::new(repo.clone(), repo.clone(), repo).into_client();
            let bound = bind_socket(&path).await.unwrap();
            let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
            let serving = tokio::task::spawn_local(serve(bound, root.client, async {
                let _ = stopped.await;
            }));
            test(path, backend).await;
            let _ = stop.send(());
            serving.await.unwrap().unwrap();
        })
        .await;
}

async fn a_person_with_a_link_and_a_role(root: &people::Client) -> (Uuid, Uuid) {
    let persons = persons_of(root).await;
    let roles = roles_of(root).await;
    let a = create(&persons, "uid-1", "Ada", "ada@x.org").await;
    let b = create(&persons, "uid-2", "A. King", "ada@x.org").await;
    let mut assign = roles.assign_request();
    write_uuid(assign.get().init_person(), Some(a));
    assign.get().set_role("member");
    assign.get().set_scope(common::CLUB);
    assign.send().promise.await.unwrap();
    let mut confirm = persons.confirm_same_request();
    write_uuid(confirm.get().init_a(), Some(a));
    write_uuid(confirm.get().init_b(), Some(b));
    confirm.send().promise.await.unwrap();
    (a, b)
}

#[tokio::test(flavor = "current_thread")]
async fn a_delete_finishes_after_its_client_went_away() {
    with_socket(PAUSE, |path, _| async move {
        let first = connect::<people::Client>(&path).await.unwrap();
        let (a, b) = a_person_with_a_link_and_a_role(&first.client).await;
        let persons = persons_of(&first.client).await;
        let mut delete = persons.delete_request();
        write_uuid(delete.get().init_id(), Some(a));
        let call = delete.send();
        tokio::time::sleep(PAUSE * 2).await;
        drop(call);
        drop(persons);
        drop(first);

        let second = connect::<people::Client>(&path).await.unwrap();
        let persons = persons_of(&second.client).await;
        let roles = roles_of(&second.client).await;
        // However busy the machine: it gets there, where without `run_to_end` it stops for good.
        for _ in 0..500 {
            let mut linked = persons.linked_to_request();
            write_uuid(linked.get().init_id(), Some(b));
            let answer = linked.send().promise.await.unwrap();
            if uuids(answer.get().unwrap().get_ids().unwrap()).is_empty() {
                break;
            }
            tokio::time::sleep(PAUSE * 4).await;
        }
        for _ in 0..500 {
            let mut of = roles.roles_of_request();
            write_uuid(of.get().init_person(), Some(a));
            let answer = of.send().promise.await.unwrap();
            if answer.get().unwrap().get_items().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(PAUSE * 4).await;
        }
        let mut get = persons.get_request();
        write_uuid(get.get().init_id(), Some(a));
        let answer = get.send().promise.await.unwrap();
        assert!(
            !answer.get().unwrap().get_found(),
            "the person is still there"
        );
        let mut linked = persons.linked_to_request();
        write_uuid(linked.get().init_id(), Some(b));
        let answer = linked.send().promise.await.unwrap();
        assert!(
            uuids(answer.get().unwrap().get_ids().unwrap()).is_empty(),
            "the link stayed"
        );
        let mut of = roles.roles_of_request();
        write_uuid(of.get().init_person(), Some(a));
        let answer = of.send().promise.await.unwrap();
        assert_eq!(
            answer.get().unwrap().get_items().unwrap().len(),
            0,
            "the role stayed"
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_import_finishes_after_its_client_went_away() {
    with_socket(PAUSE, |path, _| async move {
        let first = connect::<people::Client>(&path).await.unwrap();
        let persons = persons_of(&first.client).await;
        let cards: Vec<String> = (0..20)
            .map(|i| card(&format!("u{i}"), "N", "n@x.org"))
            .collect();
        let mut import = persons.import_text_request();
        import.get().set_media_type("application/jscontact+json");
        import
            .get()
            .set_text(format!("[{}]", cards.join(",")).as_str());
        let call = import.send();
        tokio::time::sleep(PAUSE * 3).await;
        drop(call);
        drop(persons);
        drop(first);

        // However busy the machine: it gets there, where without `run_to_end` it stops for good.
        let second = connect::<people::Client>(&path).await.unwrap();
        let persons = persons_of(&second.client).await;
        let mut stored = 0;
        for _ in 0..500 {
            let answer = persons.list_request().send().promise.await.unwrap();
            let page = answer.get().unwrap().get_page().unwrap();
            stored = page.get_items().unwrap().len();
            if stored == 20 {
                break;
            }
            tokio::time::sleep(PAUSE * 4).await;
        }
        assert_eq!(stored, 20);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_half_done_delete_is_a_problem_of_its_own_and_a_second_call_finishes_it() {
    with_socket(Duration::ZERO, |path, backend| async move {
        let connection = connect::<people::Client>(&path).await.unwrap();
        let (a, b) = a_person_with_a_link_and_a_role(&connection.client).await;
        let persons = persons_of(&connection.client).await;
        backend.fail_others.store(true, Ordering::Relaxed);
        let mut delete = persons.delete_request();
        write_uuid(delete.get().init_id(), Some(a));
        let refused = delete.send().promise.await.err().unwrap();
        let problem = Problem::from_error(&refused).unwrap();
        assert_eq!(
            problem.problem_type,
            "https://freesynergy.net/problems/people/partly-deleted"
        );
        assert_eq!(problem.status, 500);
        assert!(
            problem.detail.contains("delete again"),
            "{}",
            problem.detail
        );

        backend.fail_others.store(false, Ordering::Relaxed);
        let mut again = persons.delete_request();
        write_uuid(again.get().init_id(), Some(a));
        again.send().promise.await.unwrap();
        let mut linked = persons.linked_to_request();
        write_uuid(linked.get().init_id(), Some(b));
        let answer = linked.send().promise.await.unwrap();
        assert!(uuids(answer.get().unwrap().get_ids().unwrap()).is_empty());
    })
    .await;
}
