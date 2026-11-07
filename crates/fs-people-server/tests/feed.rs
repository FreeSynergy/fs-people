//! `subscribe` over a real socket.

mod common;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use async_trait::async_trait;
use capnp_rpc::new_client;
use common::{CLUB, card, create, persons_of, roles_of, with_root, with_server};
use fs_interfaces::Capabilities;
use fs_interfaces::common_capnp::{change, change_listener};
use fs_interfaces::rpc::{read_envelope, read_uuid, write_uuid};
use fs_people_core::{Card, Error, ListFilter, People, PeopleRepo, Result as CoreResult};
use fs_people_server::PeopleServer;
use fs_objects::{ObjectId, Store, StoredObject, TursoStore};

#[derive(Clone, Default)]
struct Seen(Rc<RefCell<Vec<String>>>);

impl change_listener::Server for Seen {
    async fn change(
        self: Rc<Self>,
        params: change_listener::ChangeParams,
    ) -> Result<(), capnp::Error> {
        let kind = match params.get()?.get_change()?.which()? {
            change::Which::Created(e) => {
                let e = read_envelope(e?)?;
                assert_eq!(e.object_type, "Profile");
                format!("created {}", e.name)
            }
            change::Which::Updated(e) => format!("updated {}", read_envelope(e?)?.name),
            change::Which::Deleted(id) => format!("deleted {}", read_uuid(id?).unwrap()),
        };
        self.0.borrow_mut().push(kind);
        Ok(())
    }

    async fn ended(
        self: Rc<Self>,
        params: change_listener::EndedParams,
        _: change_listener::EndedResults,
    ) -> Result<(), capnp::Error> {
        let problem = params.get()?.get_problem()?;
        let detail = problem.get_detail()?.to_str()?;
        self.0.borrow_mut().push(format!("ended {detail}"));
        Ok(())
    }
}

async fn settle() {
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}

#[tokio::test(flavor = "current_thread")]
async fn subscribers_see_changes_until_they_drop_the_capability() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let seen = Seen::default();
        let listener: change_listener::Client = new_client(seen.clone());
        let mut sub = root.subscribe_request();
        sub.get().set_listener(listener);
        let answer = sub.send().promise.await.unwrap();
        let subscription = answer.get().unwrap().get_subscription().unwrap();

        let id = create(&persons, "uid-1", "Watched", "w@x.org").await;
        let mut update = persons.update_request();
        write_uuid(update.get().init_id(), Some(id));
        update
            .get()
            .set_jscontact(card("uid-1", "Renamed", "w@x.org").as_str());
        update.send().promise.await.unwrap();
        let mut relate = persons.relate_request();
        write_uuid(relate.get().init_id(), Some(id));
        relate.get().set_other_uid("uid-2");
        relate.get().set_relation("friend");
        relate.send().promise.await.unwrap();
        settle().await;
        assert_eq!(
            *seen.0.borrow(),
            ["created Watched", "updated Renamed", "updated Renamed"]
        );

        let mut delete = persons.delete_request();
        write_uuid(delete.get().init_id(), Some(id));
        delete.send().promise.await.unwrap();
        settle().await;
        assert_eq!(seen.0.borrow().last(), Some(&format!("deleted {id}")));
        assert_eq!(seen.0.borrow().len(), 4, "{:?}", seen.0.borrow());

        drop(subscription);
        drop(answer);
        settle().await;
        create(&persons, "uid-3", "Unwatched", "u@x.org").await;
        settle().await;
        assert_eq!(seen.0.borrow().len(), 4, "still told after the drop");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_subscriber_does_not_learn_of_roles_or_links() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let roles = roles_of(&root).await;
        let a = create(&persons, "uid-1", "Ada", "ada@x.org").await;
        let b = create(&persons, "uid-2", "A. King", "ada@x.org").await;

        let seen = Seen::default();
        let listener: change_listener::Client = new_client(seen.clone());
        let mut sub = root.subscribe_request();
        sub.get().set_listener(listener);
        let _answer = sub.send().promise.await.unwrap();

        let mut assign = roles.assign_request();
        write_uuid(assign.get().init_person(), Some(a));
        assign.get().set_role("member");
        assign.get().set_scope(CLUB);
        let answer = assign.send().promise.await.unwrap();
        let held = answer.get().unwrap().get_assignment().unwrap();
        let held = read_uuid(held.get_id().unwrap()).unwrap();
        let mut unassign = roles.unassign_request();
        write_uuid(unassign.get().init_assignment(), Some(held));
        unassign.send().promise.await.unwrap();
        let mut confirm = persons.confirm_same_request();
        write_uuid(confirm.get().init_a(), Some(a));
        write_uuid(confirm.get().init_b(), Some(b));
        confirm.send().promise.await.unwrap();
        settle().await;
        assert!(seen.0.borrow().is_empty(), "{:?}", seen.0.borrow());
    })
    .await;
}

/// The real provider, whose reads have stopped working while its feed still runs.
struct CannotRead(Arc<PeopleRepo>);

#[async_trait]
impl People for CannotRead {
    fn capabilities(&self) -> Capabilities {
        People::capabilities(&*self.0)
    }
    async fn get(&self, _: ObjectId) -> CoreResult<Option<StoredObject>> {
        Err(Error::Stored("the disk went away".into()))
    }
    async fn list(&self, filter: &ListFilter) -> CoreResult<Vec<StoredObject>> {
        People::list(&*self.0, filter).await
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_change_that_cannot_be_read_ends_the_feed_in_words() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
            let repo = Arc::new(PeopleRepo::new(store, common::ME));
            let people = Arc::new(CannotRead(repo.clone()));
            let root = PeopleServer::new(people, repo.clone(), repo.clone()).into_client();
            with_root(root, |root| async move {
                let seen = Seen::default();
                let listener: change_listener::Client = new_client(seen.clone());
                let mut sub = root.subscribe_request();
                sub.get().set_listener(listener);
                let _answer = sub.send().promise.await.unwrap();

                repo.create(Card::individual("Unseen")).await.unwrap();
                settle().await;
                let seen = seen.0.borrow();
                assert_eq!(seen.len(), 1, "{seen:?}");
                assert!(
                    seen[0].starts_with("ended ") && seen[0].contains("disk"),
                    "{seen:?}"
                );
            })
            .await;
        })
        .await;
}
